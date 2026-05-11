use async_trait::async_trait;
use cuba_shared::{AppError, AppResult, map_inventory_db_error, map_sales_db_error};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::application::{
    CreateSalesOrderCommand, PostSalesShipmentCommand, PreviewSalesFefoPickCommand,
    SalesOrderQuery, SalesOrderRepository,
};
use crate::domain::{
    SalesDomainError, SalesLineStatus, SalesOrder, SalesOrderLine, SalesOrderStatus,
};

#[derive(Clone)]
pub struct PostgresSalesOrderRepository {
    pool: PgPool,
}

impl PostgresSalesOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn next_so_id() -> String {
        let id = Uuid::now_v7().to_string().replace('-', "");
        format!("SO-{}", &id[..17])
    }

    fn next_transaction_id(prefix: &str) -> String {
        let id = Uuid::now_v7().to_string().replace('-', "");
        format!("{prefix}-{}", &id[..17])
    }

    async fn ensure_active_customer(
        tx: &mut Transaction<'_, Postgres>,
        customer_id: &str,
    ) -> AppResult<()> {
        let row = sqlx::query(
            r#"
            SELECT is_active
            FROM mdm.mdm_customers
            WHERE customer_id = $1
            "#,
        )
        .bind(customer_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "CUSTOMER_NOT_FOUND",
                format!("客户不存在: {customer_id}"),
            ));
        };

        if !row.get::<bool, _>("is_active") {
            return Err(AppError::business(
                "CUSTOMER_INACTIVE",
                format!("客户已停用: {customer_id}"),
            ));
        }

        Ok(())
    }

    async fn ensure_active_material(
        tx: &mut Transaction<'_, Postgres>,
        material_id: &str,
    ) -> AppResult<()> {
        let row = sqlx::query(
            r#"
            SELECT status
            FROM mdm.mdm_materials
            WHERE material_id = $1
            "#,
        )
        .bind(material_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "MATERIAL_NOT_FOUND",
                format!("物料不存在: {material_id}"),
            ));
        };

        let status: String = row.get("status");
        if status != "正常" {
            return Err(AppError::business(
                "MATERIAL_INACTIVE",
                format!("物料不可用: material_id={material_id}, status={status}"),
            ));
        }

        Ok(())
    }

    async fn ensure_active_bin(
        tx: &mut Transaction<'_, Postgres>,
        bin_code: &str,
    ) -> AppResult<()> {
        let row = sqlx::query(
            r#"
            SELECT status
            FROM mdm.mdm_storage_bins
            WHERE bin_code = $1
            "#,
        )
        .bind(bin_code)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "BIN_NOT_FOUND",
                format!("货位不存在: {bin_code}"),
            ));
        };

        let status: String = row.get("status");
        if !matches!(status.as_str(), "正常" | "占用") {
            return Err(AppError::business(
                "BIN_INACTIVE",
                format!("货位不可用: bin_code={bin_code}, status={status}"),
            ));
        }

        Ok(())
    }

    async fn so_exists_for_update(
        tx: &mut Transaction<'_, Postgres>,
        so_id: &str,
    ) -> AppResult<SalesOrderStatus> {
        let row = sqlx::query(
            r#"
            SELECT status
            FROM wms.wms_sales_orders_h
            WHERE so_id = $1
            FOR UPDATE
            "#,
        )
        .bind(so_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("销售订单不存在: {so_id}")));
        };

        let status_text: String = row.get("status");
        SalesOrderStatus::from_db_text(&status_text).map_err(Self::map_domain_error)
    }

    fn map_domain_error(error: SalesDomainError) -> AppError {
        match error {
            SalesDomainError::SalesOrderLineNotFound => {
                AppError::NotFound("销售订单行不存在".to_string())
            }
            SalesDomainError::InvalidLineNo | SalesDomainError::InvalidQuantity => {
                AppError::Validation(error.to_string())
            }
            SalesDomainError::ShipmentQuantityExceeded => {
                AppError::business("SO_SHIPMENT_QTY_EXCEEDED", "发货数量超过未发数量")
            }
            SalesDomainError::EmptySalesOrder => {
                AppError::Validation("销售订单至少需要一行有效明细".to_string())
            }
            SalesDomainError::DuplicatedLineNo => {
                AppError::business("SO_LINE_DUPLICATED", "销售订单行号重复")
            }
            SalesDomainError::NoAvailableBatch => {
                AppError::business("NO_AVAILABLE_BATCH", "无可用合格批次")
            }
            SalesDomainError::InsufficientStock => {
                AppError::business("INSUFFICIENT_STOCK", "库存不足")
            }
            _ => AppError::business("SO_STATUS_INVALID", error.to_string()),
        }
    }

    async fn refresh_so_total(tx: &mut Transaction<'_, Postgres>, so_id: &str) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE wms.wms_sales_orders_h h
            SET total_amount = COALESCE((
                    SELECT SUM(line_amount)
                    FROM wms.wms_sales_orders_d d
                    WHERE d.so_id = h.so_id
                ), 0),
                total_cogs = COALESCE((
                    SELECT SUM(line_cogs)
                    FROM wms.wms_sales_orders_d d
                    WHERE d.so_id = h.so_id
                ), 0),
                updated_at = NOW()
            WHERE h.so_id = $1
            "#,
        )
        .bind(so_id)
        .execute(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        Ok(())
    }

    async fn persist_so_status(
        tx: &mut Transaction<'_, Postgres>,
        so_id: &str,
        status: SalesOrderStatus,
    ) -> AppResult<String> {
        let new_status = status.as_db_text();

        sqlx::query(
            r#"
            UPDATE wms.wms_sales_orders_h
            SET status = $2,
                updated_at = NOW()
            WHERE so_id = $1
            "#,
        )
        .bind(so_id)
        .bind(new_status)
        .execute(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        Ok(new_status.to_string())
    }

    fn sales_line_from_row(row: &sqlx::postgres::PgRow) -> AppResult<SalesOrderLine> {
        let line_status: String = row.get("line_status");

        Ok(SalesOrderLine {
            line_no: row.get("line_no"),
            material_id: row.get("material_id"),
            ordered_qty: row.get("ordered_qty"),
            shipped_qty: row.get("shipped_qty"),
            unit_price: row.get("unit_price"),
            from_bin: row.get("from_bin"),
            line_status: SalesLineStatus::from_db_text(&line_status)
                .map_err(Self::map_domain_error)?,
        })
    }

    async fn lock_sales_lines_for_update(
        tx: &mut Transaction<'_, Postgres>,
        so_id: &str,
    ) -> AppResult<Vec<SalesOrderLine>> {
        let rows = sqlx::query(
            r#"
            SELECT
                line_no,
                material_id,
                ordered_qty,
                shipped_qty,
                unit_price,
                from_bin,
                line_status
            FROM wms.wms_sales_orders_d
            WHERE so_id = $1
            ORDER BY line_no
            FOR UPDATE
            "#,
        )
        .bind(so_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sales_db_error)?;

        rows.iter()
            .map(Self::sales_line_from_row)
            .collect::<AppResult<Vec<_>>>()
    }

    async fn json_order_by_id(pool: &PgPool, so_id: &str) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT row_to_json(x) AS data
            FROM (
                SELECT
                    h.*,
                    COALESCE(
                        json_agg(d ORDER BY d.line_no)
                        FILTER (WHERE d.id IS NOT NULL),
                        '[]'::json
                    ) AS lines
                FROM wms.wms_sales_orders_h h
                LEFT JOIN wms.wms_sales_orders_d d ON d.so_id = h.so_id
                WHERE h.so_id = $1
                GROUP BY h.so_id
            ) x
            "#,
        )
        .bind(so_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sales_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("销售订单不存在: {so_id}")));
        };

        Ok(row.get::<Value, _>("data"))
    }

    async fn pick_fefo(
        tx: &mut Transaction<'_, Postgres>,
        material_id: &str,
        quantity: i32,
        from_bin: Option<&str>,
    ) -> AppResult<Vec<Value>> {
        let rows = if let Some(bin_code) = from_bin {
            Self::ensure_active_bin(tx, bin_code).await?;
            sqlx::query(
                r#"
                WITH locked_candidates AS (
                    SELECT
                        bs.batch_number,
                        bs.bin_code,
                        bs.qty::INTEGER AS available_qty,
                        b.expiry_date,
                        b.production_date
                    FROM wms.wms_bin_stock bs
                    JOIN wms.wms_batches b ON b.batch_number = bs.batch_number
                    JOIN mdm.mdm_storage_bins sb ON sb.bin_code = bs.bin_code
                    WHERE bs.material_id = $1
                      AND bs.bin_code = $3
                      AND bs.qty > 0
                      AND bs.quality_status = '合格'::mdm.quality_status
                      AND b.quality_status = '合格'::mdm.quality_status
                      AND (b.expiry_date IS NULL OR b.expiry_date >= CURRENT_DATE)
                      AND sb.status IN ('正常', '占用')
                    ORDER BY b.expiry_date NULLS LAST, b.production_date, bs.batch_number, bs.bin_code
                    FOR UPDATE OF bs SKIP LOCKED
                ), candidates AS (
                    SELECT
                        lc.*,
                        COALESCE(
                            SUM(lc.available_qty) OVER (
                                ORDER BY lc.expiry_date NULLS LAST, lc.production_date, lc.batch_number, lc.bin_code
                                ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
                            ), 0
                        )::INTEGER AS qty_before
                    FROM locked_candidates lc
                )
                SELECT
                    c.batch_number,
                    c.bin_code,
                    LEAST(c.available_qty, $2 - c.qty_before)::INTEGER AS pick_qty,
                    c.expiry_date,
                    c.available_qty
                FROM candidates c
                WHERE c.qty_before < $2
                ORDER BY c.expiry_date NULLS LAST, c.production_date, c.batch_number, c.bin_code
                "#,
            )
            .bind(material_id)
            .bind(quantity)
            .bind(bin_code)
            .fetch_all(&mut **tx)
            .await.map_err(map_sales_db_error)?
        } else {
            sqlx::query(
                r#"
                SELECT
                    batch_number,
                    bin_code,
                    pick_qty,
                    expiry_date,
                    available_qty
                FROM wms.fn_pick_batch_fefo(
                    $1,
                    $2,
                    $3,
                    '合格'::mdm.quality_status
                )
                "#,
            )
            .bind(material_id)
            .bind(quantity)
            .bind::<Option<String>>(None)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_sales_db_error)?
        };

        let mut total = 0;
        let mut picks = Vec::new();

        for row in rows {
            let pick_qty: i32 = row.get("pick_qty");
            total += pick_qty;

            picks.push(json!({
                "batch_number": row.get::<String, _>("batch_number"),
                "bin_code": row.get::<String, _>("bin_code"),
                "pick_qty": pick_qty,
                "expiry_date": row.get::<Option<Date>, _>("expiry_date"),
                "available_qty": row.get::<i32, _>("available_qty")
            }));
        }

        if total == 0 {
            return Err(AppError::business(
                "NO_AVAILABLE_BATCH",
                format!(
                    "无可用合格批次: material_id={}, required={}",
                    material_id, quantity
                ),
            ));
        }

        if total < quantity {
            return Err(AppError::business(
                "INSUFFICIENT_STOCK",
                format!(
                    "库存不足: material_id={}, required={}, available={}",
                    material_id, quantity, total
                ),
            ));
        }

        Ok(picks)
    }
}

#[async_trait]
impl SalesOrderRepository for PostgresSalesOrderRepository {
    async fn create_order(
        &self,
        command: CreateSalesOrderCommand,
        operator: String,
    ) -> AppResult<Value> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("销售订单至少需要一行明细".to_string()));
        }

        let mut tx = self.pool.begin().await.map_err(map_sales_db_error)?;
        let so_id = Self::next_so_id();

        Self::ensure_active_customer(&mut tx, &command.customer_id).await?;
        for line in &command.lines {
            Self::ensure_active_material(&mut tx, &line.material_id).await?;
            if let Some(from_bin) = &line.from_bin {
                Self::ensure_active_bin(&mut tx, from_bin).await?;
            }
        }

        sqlx::query(
            r#"
            INSERT INTO wms.wms_sales_orders_h (
                so_id,
                customer_id,
                so_date,
                delivery_date,
                status,
                created_by,
                notes
            )
            VALUES ($1, $2, CURRENT_DATE, $3, '已审批', $4, $5)
            "#,
        )
        .bind(&so_id)
        .bind(&command.customer_id)
        .bind(command.required_date)
        .bind(&operator)
        .bind(&command.remark)
        .execute(&mut *tx)
        .await
        .map_err(map_sales_db_error)?;

        for line in command.lines {
            sqlx::query(
                r#"
                INSERT INTO wms.wms_sales_orders_d (
                    so_id,
                    line_no,
                    material_id,
                    ordered_qty,
                    shipped_qty,
                    unit_price,
                    from_bin,
                    line_status
                )
                VALUES ($1, $2, $3, $4, 0, $5, $6, '待发货')
                "#,
            )
            .bind(&so_id)
            .bind(line.line_no)
            .bind(&line.material_id)
            .bind(line.ordered_qty)
            .bind(line.unit_price)
            .bind(&line.from_bin)
            .execute(&mut *tx)
            .await
            .map_err(map_sales_db_error)?;
        }

        Self::refresh_so_total(&mut tx, &so_id).await?;
        tx.commit().await.map_err(map_sales_db_error)?;

        Ok(json!({
            "so_id": so_id,
            "status": "已审批",
            "reports_stale": false
        }))
    }

    async fn list_orders(&self, query: SalesOrderQuery) -> AppResult<Value> {
        let rows = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x), '[]'::json) AS data
            FROM (
                SELECT
                    h.so_id,
                    h.customer_id,
                    c.customer_name,
                    h.so_date,
                    h.delivery_date,
                    h.total_amount,
                    h.total_cogs,
                    h.gross_margin,
                    h.currency,
                    h.status,
                    h.created_by,
                    h.created_at,
                    h.updated_at
                FROM wms.wms_sales_orders_h h
                JOIN mdm.mdm_customers c ON c.customer_id = h.customer_id
                WHERE ($1::VARCHAR IS NULL OR h.customer_id = $1)
                  AND ($2::VARCHAR IS NULL OR h.status = $2)
                  AND ($3::DATE IS NULL OR h.so_date >= $3)
                  AND ($4::DATE IS NULL OR h.so_date <= $4)
                  AND (
                        $5::VARCHAR IS NULL
                        OR EXISTS (
                            SELECT 1
                            FROM wms.wms_sales_orders_d d
                            WHERE d.so_id = h.so_id
                              AND d.material_id = $5
                        )
                  )
                ORDER BY h.created_at DESC
                LIMIT $6 OFFSET $7
            ) x
            "#,
        )
        .bind(&query.customer_id)
        .bind(&query.status)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(&query.material_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sales_db_error)?;

        Ok(rows.get::<Value, _>("data"))
    }

    async fn get_order(&self, so_id: String) -> AppResult<Value> {
        Self::json_order_by_id(&self.pool, &so_id).await
    }

    async fn preview_fefo_pick(&self, command: PreviewSalesFefoPickCommand) -> AppResult<Value> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("FEFO 预览明细不能为空".to_string()));
        }

        let mut tx = self.pool.begin().await.map_err(map_sales_db_error)?;

        sqlx::query("SET LOCAL lock_timeout = '2s'")
            .execute(&mut *tx)
            .await
            .map_err(map_sales_db_error)?;

        let status = Self::so_exists_for_update(&mut tx, &command.so_id).await?;
        status.ensure_can_ship().map_err(Self::map_domain_error)?;
        let lines = Self::lock_sales_lines_for_update(&mut tx, &command.so_id).await?;

        let mut preview_lines = Vec::new();

        for request_line in command.lines {
            let line = lines
                .iter()
                .find(|line| line.line_no == request_line.line_no)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "销售订单行不存在: so_id={}, line_no={}",
                        command.so_id, request_line.line_no
                    ))
                })?;

            if !line.can_ship(request_line.shipment_qty) {
                let mut checked_line = line.clone();
                checked_line
                    .ship(request_line.shipment_qty)
                    .map_err(Self::map_domain_error)?;
            }

            Self::ensure_active_material(&mut tx, &line.material_id).await?;
            if let Some(bin) = &line.from_bin {
                Self::ensure_active_bin(&mut tx, bin).await?;
            }

            let picks = Self::pick_fefo(
                &mut tx,
                &line.material_id,
                request_line.shipment_qty,
                line.from_bin.as_deref(),
            )
            .await?;

            preview_lines.push(json!({
                "line_no": request_line.line_no,
                "material_id": line.material_id,
                "requested_qty": request_line.shipment_qty,
                "picks": picks
            }));
        }

        tx.rollback().await.map_err(map_sales_db_error)?;

        Ok(json!({
            "so_id": command.so_id,
            "lines": preview_lines
        }))
    }

    async fn post_shipment(
        &self,
        command: PostSalesShipmentCommand,
        operator: String,
    ) -> AppResult<Value> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("发货明细不能为空".to_string()));
        }

        let posting_date: OffsetDateTime =
            command.posting_date.unwrap_or_else(OffsetDateTime::now_utc);
        let strategy = command
            .pick_strategy
            .clone()
            .unwrap_or_else(|| "FEFO".to_string())
            .to_uppercase();

        let mut tx = self.pool.begin().await.map_err(map_sales_db_error)?;

        sqlx::query("SET LOCAL lock_timeout = '5s'")
            .execute(&mut *tx)
            .await
            .map_err(map_sales_db_error)?;

        let status = Self::so_exists_for_update(&mut tx, &command.so_id).await?;
        status.ensure_can_ship().map_err(Self::map_domain_error)?;
        let mut order_lines = Self::lock_sales_lines_for_update(&mut tx, &command.so_id).await?;

        let mut transaction_results = Vec::new();

        for request_line in command.lines {
            let order_line = order_lines
                .iter_mut()
                .find(|line| line.line_no == request_line.line_no)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "销售订单行不存在: so_id={}, line_no={}",
                        command.so_id, request_line.line_no
                    ))
                })?;

            let material_id = order_line.material_id.clone();
            let default_from_bin = order_line.from_bin.clone();
            order_line
                .ship(request_line.shipment_qty)
                .map_err(Self::map_domain_error)?;

            Self::ensure_active_material(&mut tx, &material_id).await?;

            let picks = if strategy == "MANUAL" {
                let batch_number = request_line.batch_number.clone().ok_or_else(|| {
                    AppError::Validation("MANUAL 发货必须指定 batch_number".to_string())
                })?;

                let from_bin = request_line
                    .from_bin
                    .clone()
                    .or(default_from_bin.clone())
                    .ok_or_else(|| {
                        AppError::Validation("MANUAL 发货必须指定 from_bin".to_string())
                    })?;

                Self::ensure_active_bin(&mut tx, &from_bin).await?;

                vec![json!({
                    "batch_number": batch_number,
                    "bin_code": from_bin,
                    "pick_qty": request_line.shipment_qty,
                    "expiry_date": null,
                    "available_qty": request_line.shipment_qty
                })]
            } else {
                Self::pick_fefo(
                    &mut tx,
                    &material_id,
                    request_line.shipment_qty,
                    request_line
                        .from_bin
                        .as_deref()
                        .or(default_from_bin.as_deref()),
                )
                .await?
            };

            let mut shipped_total = 0;
            let mut last_transaction_id: Option<String> = None;
            let mut last_batch_number: Option<String> = None;
            let mut last_from_bin: Option<String> = None;

            for pick in picks {
                let batch_number = pick
                    .get("batch_number")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Internal("FEFO 结果缺少 batch_number".to_string()))?
                    .to_string();

                let from_bin = pick
                    .get("bin_code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Internal("FEFO 结果缺少 bin_code".to_string()))?
                    .to_string();

                let pick_qty = pick
                    .get("pick_qty")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::Internal("FEFO 结果缺少 pick_qty".to_string()))?
                    as i32;

                let transaction_id = Self::next_transaction_id("GI");

                sqlx::query(
                    r#"
                    SELECT wms.post_inventory_transaction(
                        $1,
                        '261'::wms.movement_type,
                        $2,
                        $3,
                        $4,
                        NULL,
                        $5,
                        NULL,
                        $6,
                        '合格'::mdm.quality_status,
                        $7,
                        $8,
                        $9,
                        NULL
                    )
                    "#,
                )
                .bind(&transaction_id)
                .bind(&material_id)
                .bind(pick_qty)
                .bind(&from_bin)
                .bind(&batch_number)
                .bind(&operator)
                .bind(&command.so_id)
                .bind(&command.remark)
                .bind(posting_date)
                .execute(&mut *tx)
                .await
                .map_err(map_inventory_db_error)?;

                shipped_total += pick_qty;
                last_transaction_id = Some(transaction_id.clone());
                last_batch_number = Some(batch_number.clone());
                last_from_bin = Some(from_bin.clone());

                transaction_results.push(json!({
                    "transaction_id": transaction_id,
                    "movement_type": "261",
                    "material_id": material_id,
                    "quantity": pick_qty,
                    "batch_number": batch_number,
                    "from_bin": from_bin
                }));
            }

            let map_at_shipment: Decimal = sqlx::query_scalar(
                r#"
                SELECT map_price
                FROM mdm.mdm_materials
                WHERE material_id = $1
                "#,
            )
            .bind(&material_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sales_db_error)?;

            if shipped_total != request_line.shipment_qty {
                return Err(AppError::Validation(format!(
                    "FEFO 实际分配数量与请求不一致: line_no={}, request={}, actual={}",
                    request_line.line_no, request_line.shipment_qty, shipped_total
                )));
            }

            sqlx::query(
                r#"
                UPDATE wms.wms_sales_orders_d
                SET shipped_qty = $3,
                    map_at_shipment = $6,
                    line_status = $7,
                    batch_number = COALESCE(batch_number, $4),
                    from_bin = COALESCE(from_bin, $5)
                WHERE so_id = $1
                  AND line_no = $2
                "#,
            )
            .bind(&command.so_id)
            .bind(request_line.line_no)
            .bind(order_line.shipped_qty)
            .bind(last_batch_number)
            .bind(last_from_bin)
            .bind(map_at_shipment)
            .bind(order_line.line_status.as_db_text())
            .execute(&mut *tx)
            .await
            .map_err(map_sales_db_error)?;

            let _ = last_transaction_id;
        }

        let new_status =
            SalesOrder::status_from_lines(&order_lines).map_err(Self::map_domain_error)?;
        let new_status = Self::persist_so_status(&mut tx, &command.so_id, new_status).await?;
        Self::refresh_so_total(&mut tx, &command.so_id).await?;

        tx.commit().await.map_err(map_sales_db_error)?;

        Ok(json!({
            "so_id": command.so_id,
            "status": new_status,
            "transactions": transaction_results,
            "reports_stale": true
        }))
    }

    async fn close_order(&self, so_id: String, operator: String) -> AppResult<Value> {
        let mut tx = self.pool.begin().await.map_err(map_sales_db_error)?;
        let status = Self::so_exists_for_update(&mut tx, &so_id).await?;
        status.ensure_can_close().map_err(Self::map_domain_error)?;

        let lines = Self::lock_sales_lines_for_update(&mut tx, &so_id).await?;
        for line in lines {
            if line.line_status != SalesLineStatus::Completed {
                sqlx::query(
                    r#"
                    UPDATE wms.wms_sales_orders_d
                    SET line_status = $3
                    WHERE so_id = $1
                      AND line_no = $2
                    "#,
                )
                .bind(&so_id)
                .bind(line.line_no)
                .bind(SalesLineStatus::Cancelled.as_db_text())
                .execute(&mut *tx)
                .await
                .map_err(map_sales_db_error)?;
            }
        }

        let result = sqlx::query(
            r#"
            UPDATE wms.wms_sales_orders_h
            SET status = $3,
                notes = COALESCE(notes, '') || E'\nClosed by ' || $2 || ' at ' || NOW(),
                updated_at = NOW()
            WHERE so_id = $1
            RETURNING so_id, status
            "#,
        )
        .bind(&so_id)
        .bind(&operator)
        .bind(SalesOrderStatus::Closed.as_db_text())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sales_db_error)?;

        let Some(row) = result else {
            return Err(AppError::Validation(
                "销售订单不存在，或已经完成/取消，不能关闭".to_string(),
            ));
        };

        tx.commit().await.map_err(map_sales_db_error)?;

        Ok(json!({
            "so_id": row.get::<String, _>("so_id"),
            "status": row.get::<String, _>("status")
        }))
    }
}
