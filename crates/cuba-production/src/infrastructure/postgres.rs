use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use cuba_shared::{AppError, AppResult};

use crate::application::{
    BatchGenealogyRepository, BomExplosionRepository, CompleteProductionOrderCommand,
    CreateProductionOrderCommand, CreateProductionOrderResult, ListProductionOrdersQuery,
    ListProductionVariancesQuery, PreviewBomExplosionCommand, ProductionCompleteAppResult,
    ProductionOrderRepository, ProductionPostingRepository, ProductionTransactionDto,
    ProductionVarianceRepository, ReleaseProductionOrderCommand, ReleaseProductionOrderResult,
};

#[derive(Clone)]
pub struct PostgresProductionRepository {
    pool: PgPool,
}

impl PostgresProductionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn page_limit(page_size: Option<u32>) -> i64 {
        page_size.unwrap_or(20).clamp(1, 200) as i64
    }

    fn page_offset(page: Option<u32>, page_size: Option<u32>) -> i64 {
        let page = page.unwrap_or(1).max(1);
        let limit = Self::page_limit(page_size);
        ((page - 1) as i64) * limit
    }

    fn new_order_id() -> String {
        format!("MO-{}", Utc::now().format("%Y%m%d%H%M%S%3f"))
            .chars()
            .take(30)
            .collect()
    }

    fn db_status_planned() -> &'static str {
        "计划中"
    }

    fn db_status_released() -> &'static str {
        "已下达"
    }

    fn db_status_completed() -> &'static str {
        "完成"
    }

    fn db_status_cancelled() -> &'static str {
        "取消"
    }

    async fn json_array_from_query(
        &self,
        sql: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Value> {
        let wrapped = format!(
            "SELECT COALESCE(json_agg(t), '[]'::json) AS data FROM ({sql} LIMIT $1 OFFSET $2) t"
        );

        let row = sqlx::query(&wrapped)
            .bind(limit)
            .bind(offset)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<Value, _>("data"))
    }
}

#[async_trait]
impl BomExplosionRepository for PostgresProductionRepository {
    async fn preview_bom_explosion(
        &self,
        command: PreviewBomExplosionCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x), '[]'::json) AS data
            FROM (
                SELECT
                    bom_level,
                    parent_material_id,
                    component_material_id,
                    component_name,
                    unit_qty,
                    required_qty,
                    available_qty,
                    shortage_qty,
                    is_critical,
                    CASE WHEN shortage_qty > 0 THEN true ELSE false END AS is_shortage
                FROM wms.fn_bom_explosion($1, $2, $3)
                ORDER BY bom_level, parent_material_id, component_material_id
            ) x
            "#,
        )
            .bind(&command.finished_material_id)
            .bind(command.quantity)
            .bind(&command.variant_code)
            .fetch_one(&self.pool)
            .await?;

        let components = row.get::<Value, _>("data");

        Ok(json!({
            "variant_code": command.variant_code,
            "finished_material_id": command.finished_material_id,
            "quantity": command.quantity,
            "merge_components": command.merge_components,
            "components": components
        }))
    }

    async fn get_order_components(&self, order_id: String) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x ORDER BY x.line_no), '[]'::json) AS data
            FROM (
                SELECT
                    d.line_no,
                    d.material_id AS component_material_id,
                    m.material_name AS component_material_name,
                    d.planned_qty,
                    d.actual_qty,
                    d.from_bin,
                    d.batch_number,
                    d.issue_transaction_id
                FROM wms.wms_production_orders_d d
                JOIN mdm.mdm_materials m ON m.material_id = d.material_id
                WHERE d.order_id = $1
            ) x
            "#,
        )
            .bind(order_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<Value, _>("data"))
    }
}

#[async_trait]
impl ProductionOrderRepository for PostgresProductionRepository {
    async fn create_order(
        &self,
        command: CreateProductionOrderCommand,
    ) -> AppResult<CreateProductionOrderResult> {
        let mut tx = self.pool.begin().await?;

        let order_id = Self::new_order_id();

        let exists = sqlx::query(
            r#"
            SELECT
                pv.variant_code,
                pv.base_material_id,
                pv.bom_id,
                pv.status AS variant_status,
                bh.status AS bom_status,
                wc.status AS work_center_status
            FROM mdm.mdm_product_variants pv
            JOIN mdm.mdm_bom_headers bh ON bh.bom_id = $2
            LEFT JOIN mdm.mdm_work_centers wc ON wc.work_center_id = $3
            WHERE pv.variant_code = $1
            "#,
        )
            .bind(&command.variant_code)
            .bind(&command.bom_id)
            .bind(&command.work_center_id)
            .fetch_optional(&mut *tx)
            .await?;

        let master = exists.ok_or_else(|| {
            AppError::Validation(
                "product variant, BOM, or work center does not exist".to_string(),
            )
        })?;

        let variant_status: Option<String> = master.try_get("variant_status").ok();
        let bom_status: Option<String> = master.try_get("bom_status").ok();
        let work_center_status: Option<String> = master.try_get("work_center_status").ok();

        if variant_status.as_deref() != Some("启用") && variant_status.as_deref() != Some("生效") {
            return Err(AppError::Validation(
                "product variant is not active".to_string(),
            ));
        }

        if bom_status.as_deref() != Some("生效") {
            return Err(AppError::Validation(
                "BOM is not active".to_string(),
            ));
        }

        if let Some(status) = work_center_status {
            if status != "启用" && status != "正常" {
                return Err(AppError::Validation(
                    "work center is not active".to_string(),
                ));
            }
        }

        let explosion_rows = sqlx::query(
            r#"
            SELECT
                bom_level,
                parent_material_id,
                component_material_id,
                component_name,
                unit_qty,
                required_qty,
                available_qty,
                shortage_qty,
                is_critical
            FROM wms.fn_bom_explosion($1, $2, $3)
            ORDER BY bom_level, parent_material_id, component_material_id
            "#,
        )
            .bind(&command.finished_material_id)
            .bind(command.planned_qty)
            .bind(&command.variant_code)
            .fetch_all(&mut *tx)
            .await?;

        if explosion_rows.is_empty() {
            return Err(AppError::Validation(
                "BOM explosion returned no components".to_string(),
            ));
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
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10)
            "#,
        )
            .bind(&order_id)
            .bind(&command.variant_code)
            .bind(&command.bom_id)
            .bind(&command.finished_material_id)
            .bind(&command.work_center_id)
            .bind(command.planned_qty)
            .bind(Self::db_status_planned())
            .bind(command.planned_start_date)
            .bind(command.planned_end_date)
            .bind("API")
            .execute(&mut *tx)
            .await?;

        for (idx, row) in explosion_rows.iter().enumerate() {
            let component_material_id: String = row.get("component_material_id");
            let required_qty: rust_decimal::Decimal = row.get("required_qty");

            sqlx::query(
                r#"
                INSERT INTO wms.wms_production_orders_d (
                    order_id,
                    line_no,
                    material_id,
                    planned_qty,
                    actual_qty
                )
                VALUES ($1, $2, $3, $4, 0)
                "#,
            )
                .bind(&order_id)
                .bind((idx + 1) as i32 * 10)
                .bind(component_material_id)
                .bind(required_qty.round().to_string().parse::<i32>().unwrap_or(0))
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(CreateProductionOrderResult {
            order_id,
            status: "PLANNED".to_string(),
            variant_code: command.variant_code,
            finished_material_id: command.finished_material_id,
            planned_qty: command.planned_qty,
            component_count: explosion_rows.len(),
        })
    }

    async fn list_orders(&self, query: ListProductionOrdersQuery) -> AppResult<Value> {
        let limit = Self::page_limit(query.page_size);
        let offset = Self::page_offset(query.page, query.page_size);

        let sql = r#"
            SELECT
                h.order_id,
                h.variant_code,
                h.bom_id,
                h.output_material_id,
                m.material_name AS output_material_name,
                h.work_center_id,
                wc.work_center_name,
                h.planned_quantity,
                h.actual_quantity,
                h.status,
                h.planned_start_date,
                h.planned_finish_date,
                h.actual_start_date,
                h.actual_finish_date,
                h.created_by,
                h.created_at,
                h.updated_at
            FROM wms.wms_production_orders_h h
            JOIN mdm.mdm_materials m ON m.material_id = h.output_material_id
            LEFT JOIN mdm.mdm_work_centers wc ON wc.work_center_id = h.work_center_id
            ORDER BY h.created_at DESC
        "#;

        self.json_array_from_query(sql, limit, offset).await
    }

    async fn get_order(&self, order_id: String) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT row_to_json(x) AS data
            FROM (
                SELECT
                    h.*,
                    m.material_name AS output_material_name,
                    COALESCE(
                        (
                            SELECT json_agg(d ORDER BY d.line_no)
                            FROM (
                                SELECT
                                    d.line_no,
                                    d.material_id,
                                    cm.material_name,
                                    d.batch_number,
                                    d.serial_number,
                                    d.planned_qty,
                                    d.actual_qty,
                                    d.from_bin,
                                    d.issue_transaction_id,
                                    d.created_at
                                FROM wms.wms_production_orders_d d
                                JOIN mdm.mdm_materials cm ON cm.material_id = d.material_id
                                WHERE d.order_id = h.order_id
                            ) d
                        ),
                        '[]'::json
                    ) AS lines
                FROM wms.wms_production_orders_h h
                JOIN mdm.mdm_materials m ON m.material_id = h.output_material_id
                WHERE h.order_id = $1
            ) x
            "#,
        )
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound("production order not found".to_string()));
        };

        Ok(row.get::<Value, _>("data"))
    }

    async fn release_order(
        &self,
        command: ReleaseProductionOrderCommand,
    ) -> AppResult<ReleaseProductionOrderResult> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("SET LOCAL lock_timeout = '5s'")
            .execute(&mut *tx)
            .await?;

        let current = sqlx::query(
            r#"
            SELECT status
            FROM wms.wms_production_orders_h
            WHERE order_id = $1
            FOR UPDATE
            "#,
        )
            .bind(&command.order_id)
            .fetch_optional(&mut *tx)
            .await?;

        let Some(row) = current else {
            return Err(AppError::NotFound("production order not found".to_string()));
        };

        let status: String = row.get("status");

        if status != Self::db_status_planned() {
            return Err(AppError::Validation(format!(
                "production order status invalid for release: {status}"
            )));
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
                "production order has no component lines".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE wms.wms_production_orders_h
            SET
                status = $2,
                actual_start_date = COALESCE(actual_start_date, CURRENT_DATE),
                updated_at = NOW()
            WHERE order_id = $1
            "#,
        )
            .bind(&command.order_id)
            .bind(Self::db_status_released())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(ReleaseProductionOrderResult {
            order_id: command.order_id,
            status: "RELEASED".to_string(),
        })
    }

    async fn cancel_order(
        &self,
        order_id: String,
        _remark: Option<String>,
    ) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_production_orders_h
            SET status = $2, updated_at = NOW()
            WHERE order_id = $1
              AND status IN ('计划中', '已下达', '生产中')
            RETURNING order_id, status
            "#,
        )
            .bind(&order_id)
            .bind(Self::db_status_cancelled())
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = result else {
            return Err(AppError::Validation(
                "production order cannot be cancelled".to_string(),
            ));
        };

        Ok(json!({
            "order_id": row.get::<String, _>("order_id"),
            "status": "CANCELLED"
        }))
    }

    async fn close_order(
        &self,
        order_id: String,
        _remark: Option<String>,
    ) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_production_orders_h
            SET status = $2, updated_at = NOW()
            WHERE order_id = $1
              AND status = '完成'
            RETURNING order_id, status
            "#,
        )
            .bind(&order_id)
            .bind(Self::db_status_completed())
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = result else {
            return Err(AppError::Validation(
                "production order cannot be closed".to_string(),
            ));
        };

        Ok(json!({
            "order_id": row.get::<String, _>("order_id"),
            "status": "CLOSED"
        }))
    }
}

#[async_trait]
impl ProductionPostingRepository for PostgresProductionRepository {
    async fn complete_order(
        &self,
        command: CompleteProductionOrderCommand,
    ) -> AppResult<ProductionCompleteAppResult> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("SET LOCAL lock_timeout = '10s'")
            .execute(&mut *tx)
            .await?;

        let current = sqlx::query(
            r#"
            SELECT status, planned_quantity, actual_quantity
            FROM wms.wms_production_orders_h
            WHERE order_id = $1
            FOR UPDATE
            "#,
        )
            .bind(&command.order_id)
            .fetch_optional(&mut *tx)
            .await?;

        let Some(row) = current else {
            return Err(AppError::NotFound("production order not found".to_string()));
        };

        let status: String = row.get("status");
        let planned_quantity: i32 = row.get("planned_quantity");
        let actual_quantity: i32 = row.get("actual_quantity");

        if status != Self::db_status_released() && status != "生产中" {
            return Err(AppError::Validation(format!(
                "production order status invalid for completion: {status}"
            )));
        }

        let remaining = planned_quantity - actual_quantity;
        if command.completed_qty <= 0 || command.completed_qty > remaining {
            return Err(AppError::Validation(
                "completed quantity exceeds remaining planned quantity".to_string(),
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT
                posted_action,
                posted_material_id,
                posted_batch_number,
                posted_qty,
                posted_transaction_id
            FROM wms.fn_post_production_complete(
                $1,
                $2,
                $3,
                $4,
                $5,
                '合格'::mdm.quality_status,
                COALESCE($6, NOW())
            )
            "#,
        )
            .bind(&command.order_id)
            .bind(&command.finished_batch_number)
            .bind(&command.finished_to_bin)
            .bind(command.completed_qty)
            .bind("API")
            .bind(command.posting_date)
            .fetch_all(&mut *tx)
            .await?;

        let mut finished_transaction = None;
        let mut component_transactions = Vec::new();

        for row in rows {
            let action: String = row.get("posted_action");
            let material_id: String = row.get("posted_material_id");
            let batch_number: Option<String> = row.get("posted_batch_number");
            let qty: i32 = row.get("posted_qty");
            let transaction_id: String = row.get("posted_transaction_id");

            let dto = ProductionTransactionDto {
                transaction_id,
                movement_type: if action == "入库" {
                    "101".to_string()
                } else {
                    "261".to_string()
                },
                material_id,
                quantity: qty,
                batch_number,
                from_bin: None,
                to_bin: None,
            };

            if action == "入库" {
                finished_transaction = Some(dto);
            } else {
                component_transactions.push(dto);
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

        tx.commit().await?;

        Ok(ProductionCompleteAppResult {
            order_id: command.order_id,
            status: "COMPLETED".to_string(),
            completed_qty: command.completed_qty,
            finished_transaction,
            component_transactions,
            genealogy_count,
            variance_id,
            reports_stale: true,
        })
    }
}

#[async_trait]
impl BatchGenealogyRepository for PostgresProductionRepository {
    async fn get_order_genealogy(&self, order_id: String) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x ORDER BY x.created_at), '[]'::json) AS data
            FROM (
                SELECT
                    g.id,
                    g.production_order_id,
                    g.parent_batch_number,
                    g.component_batch_number,
                    g.parent_material_id,
                    pm.material_name AS parent_material_name,
                    g.component_material_id,
                    cm.material_name AS component_material_name,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id,
                    g.created_at
                FROM wms.wms_batch_genealogy g
                JOIN mdm.mdm_materials pm ON pm.material_id = g.parent_material_id
                JOIN mdm.mdm_materials cm ON cm.material_id = g.component_material_id
                WHERE g.production_order_id = $1
            ) x
            "#,
        )
            .bind(order_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<Value, _>("data"))
    }

    async fn get_components_by_finished_batch(
        &self,
        batch_number: String,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x ORDER BY x.created_at), '[]'::json) AS data
            FROM (
                SELECT
                    g.id,
                    g.production_order_id,
                    g.parent_batch_number AS finished_batch_number,
                    g.component_batch_number,
                    g.parent_material_id AS finished_material_id,
                    pm.material_name AS finished_material_name,
                    g.component_material_id,
                    cm.material_name AS component_material_name,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id,
                    g.created_at
                FROM wms.wms_batch_genealogy g
                JOIN mdm.mdm_materials pm ON pm.material_id = g.parent_material_id
                JOIN mdm.mdm_materials cm ON cm.material_id = g.component_material_id
                WHERE g.parent_batch_number = $1
            ) x
            "#,
        )
            .bind(batch_number)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<Value, _>("data"))
    }

    async fn get_where_used_by_component_batch(
        &self,
        batch_number: String,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x ORDER BY x.created_at), '[]'::json) AS data
            FROM (
                SELECT
                    g.id,
                    g.production_order_id,
                    g.component_batch_number,
                    g.parent_batch_number AS finished_batch_number,
                    g.component_material_id,
                    cm.material_name AS component_material_name,
                    g.parent_material_id AS finished_material_id,
                    pm.material_name AS finished_material_name,
                    g.consumed_qty,
                    g.output_qty,
                    g.transaction_id,
                    g.created_at
                FROM wms.wms_batch_genealogy g
                JOIN mdm.mdm_materials pm ON pm.material_id = g.parent_material_id
                JOIN mdm.mdm_materials cm ON cm.material_id = g.component_material_id
                WHERE g.component_batch_number = $1
            ) x
            "#,
        )
            .bind(batch_number)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<Value, _>("data"))
    }
}

#[async_trait]
impl ProductionVarianceRepository for PostgresProductionRepository {
    async fn get_order_variance(&self, order_id: String) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT row_to_json(x) AS data
            FROM (
                SELECT
                    v.*,
                    m.material_name AS output_material_name
                FROM wms.wms_production_variances v
                JOIN mdm.mdm_materials m ON m.material_id = v.output_material_id
                WHERE v.order_id = $1
            ) x
            "#,
        )
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(
                "production variance not found".to_string(),
            ));
        };

        Ok(row.get::<Value, _>("data"))
    }

    async fn list_variances(
        &self,
        query: ListProductionVariancesQuery,
    ) -> AppResult<Value> {
        let limit = Self::page_limit(query.page_size);
        let offset = Self::page_offset(query.page, query.page_size);

        let sql = r#"
            SELECT
                v.*,
                m.material_name AS output_material_name
            FROM wms.wms_production_variances v
            JOIN mdm.mdm_materials m ON m.material_id = v.output_material_id
            ORDER BY v.calculated_at DESC
        "#;

        self.json_array_from_query(sql, limit, offset).await
    }
}