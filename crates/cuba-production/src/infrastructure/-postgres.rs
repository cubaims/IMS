use std::collections::BTreeMap;

use async_trait::async_trait;
use cuba_shared::{AppError, AppResult, db_error::map_production_db_error};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use time::{Date, OffsetDateTime};

use crate::{
    application::{
        BatchGenealogyRepository, BomExplosionCommand, BomExplosionRepository,
        CompleteProductionOrderCommand, CreateProductionOrderCommand, ProductionOrderQuery,
        ProductionOrderRepository, ProductionPostingRepository, ProductionVarianceQuery,
        ProductionVarianceRepository, ReleaseProductionOrderCommand,
    },
    domain::{
        BatchGenealogy, BatchNumber, BinCode, BomExplosionComponent, BomExplosionResult, BomId,
        MaterialId, ProductionCompleteResult, ProductionCompleteTransaction, ProductionOrder,
        ProductionOrderId, ProductionOrderLine, ProductionOrderStatus, ProductionVariance,
        VariantCode, WorkCenterId,
    },
};

fn map_sqlx_error(err: sqlx::Error) -> AppError {
    match err {
        sqlx::Error::Database(db_err) => {
            let message = db_err.message().to_string();
            crate::interface::error_mapping::map_db_error_text(&message)
        }
        other => AppError::Database(other),
    }
}

#[derive(Clone)]
pub struct PostgresProductionRepository {
    pool: PgPool,
}

impl PostgresProductionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn new_order_id() -> String {
        let suffix = uuid::Uuid::now_v7().to_string().replace('-', "");
        format!("MO-{}", &suffix[..20])
    }

    fn page_limit(page_size: Option<u32>) -> i64 {
        page_size.unwrap_or(20).clamp(1, 200) as i64
    }

    fn page_offset(page: Option<u32>, page_size: Option<u32>) -> i64 {
        let page = page.unwrap_or(1).max(1);
        ((page - 1) as i64) * Self::page_limit(page_size)
    }

    fn row_to_order(row: &sqlx::postgres::PgRow) -> ProductionOrder {
        let status_text: String = row.get("status");

        ProductionOrder {
            order_id: ProductionOrderId(row.get("order_id")),
            variant_code: VariantCode(
                row.get::<Option<String>, _>("variant_code")
                    .unwrap_or_default(),
            ),
            finished_material_id: MaterialId(row.get("output_material_id")),
            bom_id: BomId(row.get::<Option<String>, _>("bom_id").unwrap_or_default()),
            planned_qty: row.get("planned_quantity"),
            completed_qty: row.get("actual_quantity"),
            work_center_id: WorkCenterId(
                row.get::<Option<String>, _>("work_center_id")
                    .unwrap_or_default(),
            ),
            planned_start_date: row.get::<Option<Date>, _>("planned_start_date"),
            planned_end_date: row.get::<Option<Date>, _>("planned_finish_date"),
            status: ProductionOrderStatus::from_db_text(&status_text),
            remark: None,
            created_by: row.get::<Option<String>, _>("created_by"),
            created_at: row.get::<Option<OffsetDateTime>, _>("created_at"),
            updated_at: row.get::<Option<OffsetDateTime>, _>("updated_at"),
        }
    }

    fn row_to_line(row: &sqlx::postgres::PgRow) -> ProductionOrderLine {
        ProductionOrderLine {
            order_id: ProductionOrderId(row.get("order_id")),
            line_no: row.get("line_no"),
            component_material_id: MaterialId(row.get("material_id")),
            required_qty: row.get("planned_qty"),
            issued_qty: row.get("actual_qty"),
            source_bin: row.get::<Option<String>, _>("from_bin").map(BinCode),
            status: None,
        }
    }

    fn row_to_genealogy(row: &sqlx::postgres::PgRow) -> BatchGenealogy {
        BatchGenealogy {
            parent_batch_number: BatchNumber(row.get("parent_batch_number")),
            component_batch_number: BatchNumber(row.get("component_batch_number")),
            parent_material_id: MaterialId(row.get("parent_material_id")),
            component_material_id: MaterialId(row.get("component_material_id")),
            production_order_id: ProductionOrderId(row.get("production_order_id")),
            consumed_qty: row.get("consumed_qty"),
            output_qty: row.get("output_qty"),
            transaction_id: row.get::<Option<String>, _>("transaction_id"),
        }
    }

    fn row_to_variance(row: &sqlx::postgres::PgRow) -> ProductionVariance {
        ProductionVariance {
            order_id: ProductionOrderId(row.get("order_id")),
            variant_code: row
                .get::<Option<String>, _>("variant_code")
                .map(VariantCode),
            output_material_id: MaterialId(row.get("output_material_id")),
            planned_quantity: row.get("planned_quantity"),
            actual_quantity: row.get("actual_quantity"),
            planned_unit_cost: row.get("planned_unit_cost"),
            actual_unit_cost: row.get("actual_unit_cost"),
            planned_material_cost: row.get("planned_material_cost"),
            actual_material_cost: row.get("actual_material_cost"),
            material_variance: row.get("material_variance"),
            labor_variance: row.get("labor_variance"),
            overhead_variance: row.get("overhead_variance"),
            total_variance: row.get("total_variance"),
            variance_pct: row.get::<Option<Decimal>, _>("variance_pct"),
        }
    }
}

#[async_trait]
impl ProductionOrderRepository for PostgresProductionRepository {
    async fn create_order(
        &self,
        command: CreateProductionOrderCommand,
    ) -> AppResult<ProductionOrderId> {
        let mut tx = self.pool.begin().await?;

        let order_id = Self::new_order_id();

        let variant_exists: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT variant_code
            FROM mdm.mdm_product_variants
            WHERE variant_code = $1
              AND is_active = true
            "#,
        )
        .bind(&command.variant_code)
        .fetch_optional(&mut *tx)
        .await?;

        if variant_exists.is_none() {
            return Err(AppError::Validation("产品变体不存在或未启用".to_string()));
        }

        let bom_exists: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT bom_id
            FROM mdm.mdm_bom_headers
            WHERE bom_id = $1
              AND status = '生效'
            "#,
        )
        .bind(&command.bom_id)
        .fetch_optional(&mut *tx)
        .await?;

        if bom_exists.is_none() {
            return Err(AppError::Validation("BOM 不存在或未生效".to_string()));
        }

        let wc_exists: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT work_center_id
            FROM mdm.mdm_work_centers
            WHERE work_center_id = $1
              AND is_active = true
            "#,
        )
        .bind(&command.work_center_id)
        .fetch_optional(&mut *tx)
        .await?;

        if wc_exists.is_none() {
            return Err(AppError::Validation("工作中心不存在或未启用".to_string()));
        }

        sqlx::query(
            r#"
            INSERT INTO wms.wms_production_orders_h (
                order_id,
                variant_code,
                bom_id,
                output_material_id,
                work_center_id,
                planned_quantity,
                actual_quantity,
                status,
                planned_start_date,
                planned_finish_date,
                created_by
            )
            VALUES ($1,$2,$3,$4,$5,$6,0,'计划中',$7,$8,$9)
            "#,
        )
        .bind(&order_id)
        .bind(&command.variant_code)
        .bind(&command.bom_id)
        .bind(&command.finished_material_id)
        .bind(&command.work_center_id)
        .bind(command.planned_qty)
        .bind(command.planned_start_date)
        .bind(command.planned_end_date)
        .bind(command.created_by.clone())
        .execute(&mut *tx)
        .await?;

        let components = sqlx::query(
            r#"
            SELECT *
            FROM wms.fn_bom_explosion($1, $2, $3)
            "#,
        )
        .bind(&command.finished_material_id)
        .bind(Decimal::from(command.planned_qty))
        .bind(&command.variant_code)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_production_db_error)?;

        if components.is_empty() {
            return Err(AppError::Validation(
                "BOM 爆炸结果为空，不能创建生产订单".to_string(),
            ));
        }

        for (idx, row) in components.iter().enumerate() {
            let material_id: String = row.get("component_material_id");
            let required_qty: Decimal = row.get("required_qty");
            let planned_qty = required_qty.ceil().to_string().parse::<i32>().unwrap_or(0);

            if planned_qty <= 0 {
                return Err(AppError::Validation(format!(
                    "组件 {material_id} 的需求数量无效"
                )));
            }

            sqlx::query(
                r#"
        INSERT INTO wms.wms_production_orders_d (
            order_id,
            line_no,
            material_id,
            planned_qty,
            actual_qty
        )
        VALUES ($1,$2,$3,$4,0)
        "#,
            )
            .bind(&order_id)
            .bind((idx + 1) as i32 * 10)
            .bind(material_id)
            .bind(planned_qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(ProductionOrderId(order_id))
    }

    async fn find_by_id(&self, order_id: &str) -> AppResult<ProductionOrder> {
        let row = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_production_orders_h
            WHERE order_id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        Ok(Self::row_to_order(&row))
    }

    async fn list(&self, query: ProductionOrderQuery) -> AppResult<Vec<ProductionOrder>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            SELECT *
            FROM wms.wms_production_orders_h
            WHERE 1 = 1
            "#,
        );

        if let Some(order_id) = query.order_id {
            builder.push(" AND order_id = ");
            builder.push_bind(order_id);
        }

        if let Some(variant_code) = query.variant_code {
            builder.push(" AND variant_code = ");
            builder.push_bind(variant_code);
        }

        if let Some(finished_material_id) = query.finished_material_id {
            builder.push(" AND output_material_id = ");
            builder.push_bind(finished_material_id);
        }

        if let Some(status) = query.status {
            builder.push(" AND status = ");
            builder.push_bind(status);
        }

        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(Self::page_limit(query.page_size));
        builder.push(" OFFSET ");
        builder.push_bind(Self::page_offset(query.page, query.page_size));

        let rows = builder.build().fetch_all(&self.pool).await?;

        Ok(rows.iter().map(Self::row_to_order).collect())
    }

    async fn list_lines(&self, order_id: &str) -> AppResult<Vec<ProductionOrderLine>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_production_orders_d
            WHERE order_id = $1
            ORDER BY line_no
            "#,
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_line).collect())
    }

    async fn release(&self, command: ReleaseProductionOrderCommand) -> AppResult<ProductionOrder> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_production_orders_h
            WHERE order_id = $1
            FOR UPDATE
            "#,
        )
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        let status_text: String = row.get("status");
        let status = ProductionOrderStatus::from_db_text(&status_text);

        if !status.can_release() {
            return Err(AppError::Validation(
                "只有计划中生产订单可以下达".to_string(),
            ));
        }

        let component_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_production_orders_d
            WHERE order_id = $1
            "#,
        )
        .bind(&command.order_id)
        .fetch_one(&mut *tx)
        .await?;

        if component_count == 0 {
            return Err(AppError::Validation(
                "生产订单没有组件行，不能下达".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE wms.wms_production_orders_h
            SET status = '已下达',
                actual_start_date = COALESCE(actual_start_date, CURRENT_DATE),
                updated_at = NOW()
            WHERE order_id = $1
            "#,
        )
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.find_by_id(&command.order_id).await
    }

    async fn cancel(
        &self,
        order_id: &str,
        _operator: Option<String>,
    ) -> AppResult<ProductionOrder> {
        sqlx::query(
            r#"
            UPDATE wms.wms_production_orders_h
            SET status = '取消',
                updated_at = NOW()
            WHERE order_id = $1
              AND status IN ('计划中','已下达')
            "#,
        )
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(order_id).await
    }

    async fn close(&self, order_id: &str, _operator: Option<String>) -> AppResult<ProductionOrder> {
        sqlx::query(
            r#"
            UPDATE wms.wms_production_orders_h
            SET status = '完成',
                actual_finish_date = COALESCE(actual_finish_date, CURRENT_DATE),
                updated_at = NOW()
            WHERE order_id = $1
              AND status = '完成'
            "#,
        )
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(order_id).await
    }
}

#[async_trait]
impl BomExplosionRepository for PostgresProductionRepository {
    async fn explode(&self, command: BomExplosionCommand) -> AppResult<BomExplosionResult> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM wms.fn_bom_explosion($1, $2, $3)
            "#,
        )
        .bind(&command.finished_material_id)
        .bind(Decimal::from(command.quantity))
        .bind(&command.variant_code)
        .fetch_all(&self.pool)
        .await?;

        let merge = command.merge_components.unwrap_or(true);

        let mut components = Vec::new();

        for row in rows {
            let component = BomExplosionComponent {
                level: row.get("bom_level"),
                parent_material_id: MaterialId(row.get("parent_material_id")),
                component_material_id: MaterialId(row.get("component_material_id")),
                component_name: row.get::<Option<String>, _>("component_name"),
                quantity_per: row.get("unit_qty"),
                required_qty: row.get("required_qty"),
                available_qty: Decimal::from(row.get::<i32, _>("available_qty")),
                net_requirement_qty: row.get("shortage_qty"),
                is_shortage: row.get::<Decimal, _>("shortage_qty") > Decimal::ZERO,
            };

            components.push(component);
        }

        if merge {
            let mut merged: BTreeMap<String, BomExplosionComponent> = BTreeMap::new();

            for component in components {
                let key = component.component_material_id.0.clone();

                merged
                    .entry(key)
                    .and_modify(|existing| {
                        existing.required_qty += component.required_qty;
                        existing.net_requirement_qty += component.net_requirement_qty;
                        existing.is_shortage = existing.is_shortage || component.is_shortage;
                    })
                    .or_insert(component);
            }

            components = merged.into_values().collect();
        }

        Ok(BomExplosionResult {
            variant_code: command.variant_code.map(VariantCode),
            finished_material_id: MaterialId(command.finished_material_id),
            quantity: command.quantity,
            merge_components: merge,
            components,
        })
    }
}

#[async_trait]
impl ProductionPostingRepository for PostgresProductionRepository {
    async fn complete_order(
        &self,
        command: CompleteProductionOrderCommand,
    ) -> AppResult<ProductionCompleteResult> {
        let mut tx = self.pool.begin().await?;

        let order_row = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_production_orders_h
            WHERE order_id = $1
            FOR UPDATE
            "#,
        )
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        let status_text: String = order_row.get("status");
        let status = ProductionOrderStatus::from_db_text(&status_text);

        if !status.can_complete() {
            return Err(AppError::Validation(
                "只有已下达或生产中的订单可以完工".to_string(),
            ));
        }

        let planned_qty: i32 = order_row.get("planned_quantity");
        let actual_qty: i32 = order_row.get("actual_quantity");

        if command.completed_qty <= 0 {
            return Err(AppError::Validation("完工数量必须大于 0".to_string()));
        }

        if actual_qty + command.completed_qty > planned_qty {
            return Err(AppError::Validation("完工数量超过计划剩余数量".to_string()));
        }

        let posting_time = command.posting_date.unwrap_or_else(OffsetDateTime::now_utc);
        let operator = command
            .operator
            .clone()
            .unwrap_or_else(|| "API".to_string());

        let posted_rows = sqlx::query(
            r#"
            SELECT *
            FROM wms.fn_post_production_complete(
                $1,
                $2,
                $3,
                $4,
                $5,
                '合格'::mdm.quality_status,
                $6
            )
            "#,
        )
        .bind(&command.order_id)
        .bind(&command.finished_batch_number)
        .bind(&command.finished_to_bin)
        .bind(command.completed_qty)
        .bind(&operator)
        .bind(posting_time)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_production_db_error)?;

        let mut finished_transaction = None;
        let mut component_transactions = Vec::new();

        for row in posted_rows {
            let action: String = row.get("posted_action");

            let item = ProductionCompleteTransaction {
                transaction_id: row.get("posted_transaction_id"),
                movement_type: if action == "入库" {
                    "101".to_string()
                } else {
                    "261".to_string()
                },
                material_id: MaterialId(row.get("posted_material_id")),
                quantity: row.get("posted_qty"),
                batch_number: row
                    .get::<Option<String>, _>("posted_batch_number")
                    .map(BatchNumber),
                from_bin: None,
                to_bin: None,
            };

            if action == "入库" {
                finished_transaction = Some(item);
            } else {
                component_transactions.push(item);
            }
        }

        let genealogy_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_batch_genealogy
            WHERE production_order_id = $1
            "#,
        )
        .bind(&command.order_id)
        .fetch_one(&mut *tx)
        .await?;

        let variance_id: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT variance_id
            FROM wms.wms_production_variances
            WHERE order_id = $1
            "#,
        )
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await?;

        let updated_status = if actual_qty + command.completed_qty >= planned_qty {
            ProductionOrderStatus::Completed
        } else {
            ProductionOrderStatus::InProduction
        };

        sqlx::query(
            r#"
    UPDATE wms.wms_production_orders_h
    SET status = $2,
        actual_quantity = LEAST(planned_quantity, actual_quantity + $3),
        actual_finish_date = CASE
            WHEN actual_quantity + $3 >= planned_quantity THEN COALESCE(actual_finish_date, CURRENT_DATE)
            ELSE actual_finish_date
        END,
        updated_at = NOW()
    WHERE order_id = $1
    "#,
        )
            .bind(&command.order_id)
            .bind(updated_status.as_db_text())
            .bind(command.completed_qty)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(ProductionCompleteResult {
            order_id: ProductionOrderId(command.order_id),
            status: updated_status,
            completed_qty: command.completed_qty,
            finished_transaction,
            component_transactions,
            genealogy_count,
            variance_id: variance_id.map(|id| id.to_string()),
            reports_stale: true,
        })
    }
}

#[async_trait]
impl BatchGenealogyRepository for PostgresProductionRepository {
    async fn find_by_order_id(&self, order_id: &str) -> AppResult<Vec<BatchGenealogy>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_batch_genealogy
            WHERE production_order_id = $1
            ORDER BY created_at, component_batch_number
            "#,
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_genealogy).collect())
    }

    async fn find_components_by_finished_batch(
        &self,
        batch_number: &str,
    ) -> AppResult<Vec<BatchGenealogy>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_batch_genealogy
            WHERE parent_batch_number = $1
            ORDER BY created_at, component_batch_number
            "#,
        )
        .bind(batch_number)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_genealogy).collect())
    }

    async fn find_where_used_by_component_batch(
        &self,
        batch_number: &str,
    ) -> AppResult<Vec<BatchGenealogy>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_batch_genealogy
            WHERE component_batch_number = $1
            ORDER BY created_at, parent_batch_number
            "#,
        )
        .bind(batch_number)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_genealogy).collect())
    }
}

#[async_trait]
impl ProductionVarianceRepository for PostgresProductionRepository {
    async fn find_by_order_id(&self, order_id: &str) -> AppResult<ProductionVariance> {
        let row = sqlx::query(
            r#"
            SELECT *
            FROM wms.wms_production_variances
            WHERE order_id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("生产成本差异不存在".to_string()))?;

        Ok(Self::row_to_variance(&row))
    }

    async fn list(&self, query: ProductionVarianceQuery) -> AppResult<Vec<ProductionVariance>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            SELECT *
            FROM wms.wms_production_variances
            WHERE 1 = 1
            "#,
        );

        if let Some(order_id) = query.order_id {
            builder.push(" AND order_id = ");
            builder.push_bind(order_id);
        }

        if let Some(variant_code) = query.variant_code {
            builder.push(" AND variant_code = ");
            builder.push_bind(variant_code);
        }

        if query.only_over_budget.unwrap_or(false) {
            builder.push(" AND total_variance > 0 ");
        }

        builder.push(" ORDER BY calculated_at DESC LIMIT ");
        builder.push_bind(Self::page_limit(query.page_size));
        builder.push(" OFFSET ");
        builder.push_bind(Self::page_offset(query.page, query.page_size));

        let rows = builder.build().fetch_all(&self.pool).await?;

        Ok(rows.iter().map(Self::row_to_variance).collect())
    }
}
