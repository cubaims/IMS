use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use cuba_shared::{AppError, AppResult, map_inventory_db_error};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::application::{
    CreateSalesOrderCommand, PostSalesShipmentCommand, PreviewSalesFefoPickCommand,
    SalesOrderQuery, SalesOrderRepository,
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

    async fn so_exists_for_update(
        tx: &mut Transaction<'_, Postgres>,
        so_id: &str,
    ) -> AppResult<String> {
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
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("销售订单不存在: {so_id}")));
        };

        Ok(row.get::<String, _>("status"))
    }

    fn validate_shipment_status(status: &str) -> AppResult<()> {
        match status {
            "已审批" | "部分发货" => Ok(()),

            "完成" => Err(AppError::business(
                "SO_STATUS_INVALID",
                "销售订单已完成，不能重复发货",
            )),

            "取消" => Err(AppError::business(
                "SO_STATUS_INVALID",
                "销售订单已取消，不能发货",
            )),

            "草稿" => Err(AppError::business(
                "SO_STATUS_INVALID",
                "销售订单仍为草稿，不能发货",
            )),

            other => Err(AppError::business(
                "SO_STATUS_INVALID",
                format!("销售订单状态不允许发货: {other}"),
            )),
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
        .await?;

        Ok(())
    }

    async fn refresh_so_status(
        tx: &mut Transaction<'_, Postgres>,
        so_id: &str,
    ) -> AppResult<String> {
        let row = sqlx::query(
            r#"
            SELECT
                SUM(ordered_qty) AS ordered_qty,
                SUM(shipped_qty) AS shipped_qty
            FROM wms.wms_sales_orders_d
            WHERE so_id = $1
              AND line_status <> '取消'
            "#,
        )
        .bind(so_id)
        .fetch_one(&mut **tx)
        .await?;

        let ordered_qty: Option<i64> = row.get("ordered_qty");
        let shipped_qty: Option<i64> = row.get("shipped_qty");

        let ordered_qty = ordered_qty.unwrap_or(0);
        let shipped_qty = shipped_qty.unwrap_or(0);

        let new_status = if ordered_qty > 0 && shipped_qty >= ordered_qty {
            "完成"
        } else if shipped_qty > 0 {
            "部分发货"
        } else {
            "已审批"
        };

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
        .await?;

        Ok(new_status.to_string())
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
        .await?;

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
        let from_zone = match from_bin {
            Some(bin) => {
                let row = sqlx::query(
                    r#"
                    SELECT zone
                    FROM mdm.mdm_storage_bins
                    WHERE bin_code = $1
                    "#,
                )
                .bind(bin)
                .fetch_optional(&mut **tx)
                .await?;

                row.map(|r| r.get::<String, _>("zone"))
            }
            None => None,
        };

        let rows = sqlx::query(
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
        .bind(from_zone)
        .fetch_all(&mut **tx)
        .await?;

        let mut total = 0;
        let mut picks = Vec::new();

        for row in rows {
            let pick_qty: i32 = row.get("pick_qty");
            total += pick_qty;

            picks.push(json!({
                "batch_number": row.get::<String, _>("batch_number"),
                "bin_code": row.get::<String, _>("bin_code"),
                "pick_qty": pick_qty,
                "expiry_date": row.get::<Option<NaiveDate>, _>("expiry_date"),
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

        let mut tx = self.pool.begin().await?;
        let so_id = Self::next_so_id();

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
        .await?;

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
            .await?;
        }

        Self::refresh_so_total(&mut tx, &so_id).await?;
        tx.commit().await?;

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
        .await?;

        Ok(rows.get::<Value, _>("data"))
    }

    async fn get_order(&self, so_id: String) -> AppResult<Value> {
        Self::json_order_by_id(&self.pool, &so_id).await
    }

    async fn preview_fefo_pick(&self, command: PreviewSalesFefoPickCommand) -> AppResult<Value> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("FEFO 预览明细不能为空".to_string()));
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query("SET LOCAL lock_timeout = '2s'")
            .execute(&mut *tx)
            .await?;

        let _status = Self::so_exists_for_update(&mut tx, &command.so_id).await?;

        let mut preview_lines = Vec::new();

        for request_line in command.lines {
            let line = sqlx::query(
                r#"
                SELECT
                    line_no,
                    material_id,
                    open_qty,
                    from_bin
                FROM wms.wms_sales_orders_d
                WHERE so_id = $1
                  AND line_no = $2
                FOR UPDATE
                "#,
            )
            .bind(&command.so_id)
            .bind(request_line.line_no)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(line) = line else {
                return Err(AppError::NotFound(format!(
                    "销售订单行不存在: so_id={}, line_no={}",
                    command.so_id, request_line.line_no
                )));
            };

            let material_id: String = line.get("material_id");
            let open_qty: i32 = line.get("open_qty");
            let from_bin: Option<String> = line.get("from_bin");

            if request_line.shipment_qty <= 0 {
                return Err(AppError::Validation("发货数量必须大于 0".to_string()));
            }

            if request_line.shipment_qty > open_qty {
                return Err(AppError::business(
                    "SO_SHIPMENT_QTY_EXCEEDED",
                    format!(
                        "发货数量超过未发数量: line_no={}, shipment_qty={}, open_qty={}",
                        request_line.line_no, request_line.shipment_qty, open_qty
                    ),
                ));
            }

            let picks = Self::pick_fefo(
                &mut tx,
                &material_id,
                request_line.shipment_qty,
                from_bin.as_deref(),
            )
            .await?;

            preview_lines.push(json!({
                "line_no": request_line.line_no,
                "material_id": material_id,
                "requested_qty": request_line.shipment_qty,
                "picks": picks
            }));
        }

        tx.rollback().await?;

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

        let posting_date: DateTime<Utc> = command.posting_date.unwrap_or_else(Utc::now);
        let strategy = command
            .pick_strategy
            .clone()
            .unwrap_or_else(|| "FEFO".to_string())
            .to_uppercase();

        let mut tx = self.pool.begin().await?;

        sqlx::query("SET LOCAL lock_timeout = '5s'")
            .execute(&mut *tx)
            .await?;

        let status = Self::so_exists_for_update(&mut tx, &command.so_id).await?;
        Self::validate_shipment_status(&status)?;

        let mut transaction_results = Vec::new();

        for request_line in command.lines {
            let line = sqlx::query(
                r#"
                SELECT
                    id,
                    line_no,
                    material_id,
                    ordered_qty,
                    shipped_qty,
                    open_qty,
                    unit_price,
                    from_bin,
                    line_status
                FROM wms.wms_sales_orders_d
                WHERE so_id = $1
                  AND line_no = $2
                FOR UPDATE
                "#,
            )
            .bind(&command.so_id)
            .bind(request_line.line_no)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(line) = line else {
                return Err(AppError::NotFound(format!(
                    "销售订单行不存在: so_id={}, line_no={}",
                    command.so_id, request_line.line_no
                )));
            };

            let material_id: String = line.get("material_id");
            let open_qty: i32 = line.get("open_qty");
            let default_from_bin: Option<String> = line.get("from_bin");

            if request_line.shipment_qty <= 0 {
                return Err(AppError::Validation("发货数量必须大于 0".to_string()));
            }

            if request_line.shipment_qty > open_qty {
                return Err(AppError::business(
                    "SO_SHIPMENT_QTY_EXCEEDED",
                    format!(
                        "发货数量超过未发数量: line_no={}, shipment_qty={}, open_qty={}",
                        request_line.line_no, request_line.shipment_qty, open_qty
                    ),
                ));
            }

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

            if shipped_total != request_line.shipment_qty {
                return Err(AppError::Validation(format!(
                    "FEFO 实际分配数量与请求不一致: line_no={}, request={}, actual={}",
                    request_line.line_no, request_line.shipment_qty, shipped_total
                )));
            }

            sqlx::query(
                r#"
                UPDATE wms.wms_sales_orders_d
                SET shipped_qty = shipped_qty + $3,
                    line_status = CASE
                        WHEN shipped_qty + $3 >= ordered_qty THEN '完成'
                        ELSE '部分发货'
                    END,
                    batch_number = COALESCE(batch_number, $4),
                    from_bin = COALESCE(from_bin, $5)
                WHERE so_id = $1
                  AND line_no = $2
                "#,
            )
            .bind(&command.so_id)
            .bind(request_line.line_no)
            .bind(shipped_total)
            .bind(last_batch_number)
            .bind(last_from_bin)
            .execute(&mut *tx)
            .await?;

            let _ = last_transaction_id;
        }

        let new_status = Self::refresh_so_status(&mut tx, &command.so_id).await?;
        Self::refresh_so_total(&mut tx, &command.so_id).await?;

        tx.commit().await?;

        Ok(json!({
            "so_id": command.so_id,
            "status": new_status,
            "transactions": transaction_results,
            "reports_stale": true
        }))
    }

    async fn close_order(&self, so_id: String, operator: String) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_sales_orders_h
            SET status = '取消',
                notes = COALESCE(notes, '') || E'\nClosed by ' || $2 || ' at ' || NOW(),
                updated_at = NOW()
            WHERE so_id = $1
              AND status <> '完成'
              AND status <> '取消'
            RETURNING so_id, status
            "#,
        )
        .bind(&so_id)
        .bind(&operator)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = result else {
            return Err(AppError::Validation(
                "销售订单不存在，或已经完成/取消，不能关闭".to_string(),
            ));
        };

        Ok(json!({
            "so_id": row.get::<String, _>("so_id"),
            "status": row.get::<String, _>("status")
        }))
    }
}
