use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cuba_shared::{AppError, AppResult, map_inventory_db_error};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderQuery,
    PurchaseOrderRepository,
};

#[derive(Clone)]
pub struct PostgresPurchaseOrderRepository {
    pool: PgPool,
}

impl PostgresPurchaseOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn next_po_id() -> String {
        let id = Uuid::now_v7().to_string().replace('-', "");
        format!("PO-{}", &id[..17])
    }

    fn next_transaction_id(prefix: &str) -> String {
        let id = Uuid::now_v7().to_string().replace('-', "");
        format!("{prefix}-{}", &id[..17])
    }

    async fn po_exists_for_update(
        tx: &mut Transaction<'_, Postgres>,
        po_id: &str,
    ) -> AppResult<String> {
        let row = sqlx::query(
            r#"
            SELECT status
            FROM wms.wms_purchase_orders_h
            WHERE po_id = $1
            FOR UPDATE
            "#,
        )
        .bind(po_id)
        .fetch_optional(&mut **tx)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("采购订单不存在: {po_id}")));
        };

        Ok(row.get::<String, _>("status"))
    }

    fn validate_receipt_status(status: &str) -> AppResult<()> {
        match status {
            "已审批" | "部分到货" => Ok(()),

            "完成" => Err(AppError::business(
                "PO_STATUS_INVALID",
                "采购订单已完成，不能重复收货",
            )),

            "取消" => Err(AppError::business(
                "PO_STATUS_INVALID",
                "采购订单已取消，不能收货",
            )),

            "草稿" => Err(AppError::business(
                "PO_STATUS_INVALID",
                "采购订单仍为草稿，不能收货",
            )),

            other => Err(AppError::business(
                "PO_STATUS_INVALID",
                format!("采购订单状态不允许收货: {other}"),
            )),
        }
    }

    async fn refresh_po_total(tx: &mut Transaction<'_, Postgres>, po_id: &str) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE wms.wms_purchase_orders_h h
            SET total_amount = COALESCE((
                SELECT SUM(line_amount)
                FROM wms.wms_purchase_orders_d d
                WHERE d.po_id = h.po_id
            ), 0),
            updated_at = NOW()
            WHERE h.po_id = $1
            "#,
        )
        .bind(po_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn refresh_po_status(
        tx: &mut Transaction<'_, Postgres>,
        po_id: &str,
    ) -> AppResult<String> {
        let row = sqlx::query(
            r#"
            SELECT
                SUM(ordered_qty) AS ordered_qty,
                SUM(received_qty) AS received_qty
            FROM wms.wms_purchase_orders_d
            WHERE po_id = $1
              AND line_status <> '取消'
            "#,
        )
        .bind(po_id)
        .fetch_one(&mut **tx)
        .await?;

        let ordered_qty: Option<i64> = row.get("ordered_qty");
        let received_qty: Option<i64> = row.get("received_qty");

        let ordered_qty = ordered_qty.unwrap_or(0);
        let received_qty = received_qty.unwrap_or(0);

        let new_status = if ordered_qty > 0 && received_qty >= ordered_qty {
            "完成"
        } else if received_qty > 0 {
            "部分到货"
        } else {
            "已审批"
        };

        sqlx::query(
            r#"
            UPDATE wms.wms_purchase_orders_h
            SET status = $2,
                updated_at = NOW()
            WHERE po_id = $1
            "#,
        )
        .bind(po_id)
        .bind(new_status)
        .execute(&mut **tx)
        .await?;

        Ok(new_status.to_string())
    }

    async fn ensure_receipt_batch(
        tx: &mut Transaction<'_, Postgres>,
        batch_number: &str,
        material_id: &str,
        to_bin: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO wms.wms_batches (
                batch_number,
                material_id,
                production_date,
                expiry_date,
                quality_grade,
                current_stock,
                current_bin,
                quality_status
            )
            VALUES (
                $1,
                $2,
                CURRENT_DATE,
                CURRENT_DATE + INTERVAL '365 days',
                'A',
                0,
                $3,
                '合格'
            )
            ON CONFLICT (batch_number) DO NOTHING
            "#,
        )
        .bind(batch_number)
        .bind(material_id)
        .bind(to_bin)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn json_order_by_id(pool: &PgPool, po_id: &str) -> AppResult<Value> {
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
                FROM wms.wms_purchase_orders_h h
                LEFT JOIN wms.wms_purchase_orders_d d ON d.po_id = h.po_id
                WHERE h.po_id = $1
                GROUP BY h.po_id
            ) x
            "#,
        )
        .bind(po_id)
        .fetch_optional(pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("采购订单不存在: {po_id}")));
        };

        Ok(row.get::<Value, _>("data"))
    }
}

#[async_trait]
impl PurchaseOrderRepository for PostgresPurchaseOrderRepository {
    async fn create_order(
        &self,
        command: CreatePurchaseOrderCommand,
        operator: String,
    ) -> AppResult<Value> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("采购订单至少需要一行明细".to_string()));
        }

        let mut tx = self.pool.begin().await?;
        let po_id = Self::next_po_id();

        sqlx::query(
            r#"
            INSERT INTO wms.wms_purchase_orders_h (
                po_id,
                supplier_id,
                po_date,
                expected_date,
                status,
                created_by,
                notes
            )
            VALUES ($1, $2, CURRENT_DATE, $3, '已审批', $4, $5)
            "#,
        )
        .bind(&po_id)
        .bind(&command.supplier_id)
        .bind(command.expected_date)
        .bind(&operator)
        .bind(&command.remark)
        .execute(&mut *tx)
        .await?;

        for line in command.lines {
            sqlx::query(
                r#"
                INSERT INTO wms.wms_purchase_orders_d (
                    po_id,
                    line_no,
                    material_id,
                    ordered_qty,
                    received_qty,
                    unit_price,
                    expected_bin,
                    line_status
                )
                VALUES ($1, $2, $3, $4, 0, $5, $6, '待到货')
                "#,
            )
            .bind(&po_id)
            .bind(line.line_no)
            .bind(&line.material_id)
            .bind(line.ordered_qty)
            .bind(line.unit_price)
            .bind(&line.expected_bin)
            .execute(&mut *tx)
            .await?;
        }

        Self::refresh_po_total(&mut tx, &po_id).await?;
        tx.commit().await?;

        Ok(json!({
            "po_id": po_id,
            "status": "已审批",
            "reports_stale": false
        }))
    }

    async fn list_orders(&self, query: PurchaseOrderQuery) -> AppResult<Value> {
        let rows = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x), '[]'::json) AS data
            FROM (
                SELECT
                    h.po_id,
                    h.supplier_id,
                    s.supplier_name,
                    h.po_date,
                    h.expected_date,
                    h.total_amount,
                    h.currency,
                    h.status,
                    h.created_by,
                    h.created_at,
                    h.updated_at
                FROM wms.wms_purchase_orders_h h
                JOIN mdm.mdm_suppliers s ON s.supplier_id = h.supplier_id
                WHERE ($1::VARCHAR IS NULL OR h.supplier_id = $1)
                  AND ($2::VARCHAR IS NULL OR h.status = $2)
                  AND ($3::DATE IS NULL OR h.po_date >= $3)
                  AND ($4::DATE IS NULL OR h.po_date <= $4)
                  AND (
                        $5::VARCHAR IS NULL
                        OR EXISTS (
                            SELECT 1
                            FROM wms.wms_purchase_orders_d d
                            WHERE d.po_id = h.po_id
                              AND d.material_id = $5
                        )
                  )
                ORDER BY h.created_at DESC
                LIMIT $6 OFFSET $7
            ) x
            "#,
        )
        .bind(&query.supplier_id)
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

    async fn get_order(&self, po_id: String) -> AppResult<Value> {
        Self::json_order_by_id(&self.pool, &po_id).await
    }

    async fn post_receipt(
        &self,
        command: PostPurchaseReceiptCommand,
        operator: String,
    ) -> AppResult<Value> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("收货明细不能为空".to_string()));
        }

        let posting_date: DateTime<Utc> = command.posting_date.unwrap_or_else(Utc::now);
        let mut tx = self.pool.begin().await?;

        sqlx::query("SET LOCAL lock_timeout = '5s'")
            .execute(&mut *tx)
            .await?;

        let status = Self::po_exists_for_update(&mut tx, &command.po_id).await?;
        Self::validate_receipt_status(&status)?;

        let mut transaction_results = Vec::new();

        for receipt_line in command.lines {
            let line = sqlx::query(
                r#"
                SELECT
                    id,
                    line_no,
                    material_id,
                    ordered_qty,
                    received_qty,
                    open_qty,
                    unit_price,
                    expected_bin,
                    line_status
                FROM wms.wms_purchase_orders_d
                WHERE po_id = $1
                  AND line_no = $2
                FOR UPDATE
                "#,
            )
            .bind(&command.po_id)
            .bind(receipt_line.line_no)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(line) = line else {
                return Err(AppError::NotFound(format!(
                    "采购订单行不存在: po_id={}, line_no={}",
                    command.po_id, receipt_line.line_no
                )));
            };

            let material_id: String = line.get("material_id");
            let open_qty: i32 = line.get("open_qty");
            let unit_price: Decimal = line.get("unit_price");
            let expected_bin: Option<String> = line.get("expected_bin");

            if receipt_line.receipt_qty <= 0 {
                return Err(AppError::Validation("收货数量必须大于 0".to_string()));
            }

            if receipt_line.receipt_qty > open_qty {
                return Err(AppError::business(
                    "PO_RECEIPT_QTY_EXCEEDED",
                    format!(
                        "收货数量超过未收数量: line_no={}, receipt_qty={}, open_qty={}",
                        receipt_line.line_no, receipt_line.receipt_qty, open_qty
                    ),
                ));
            }

            let to_bin = receipt_line
                .to_bin
                .clone()
                .or(expected_bin)
                .ok_or_else(|| AppError::Validation("收货必须指定目标货位".to_string()))?;

            Self::ensure_receipt_batch(
                &mut tx,
                &receipt_line.batch_number,
                &material_id,
                Some(&to_bin),
            )
            .await?;

            let transaction_id = Self::next_transaction_id("GR");

            sqlx::query(
                r#"
                SELECT wms.post_inventory_transaction(
                    $1,
                    '101'::wms.movement_type,
                    $2,
                    $3,
                    NULL,
                    $4,
                    $5,
                    NULL,
                    $6,
                    '合格'::mdm.quality_status,
                    $7,
                    $8,
                    $9,
                    $10
                )
                "#,
            )
            .bind(&transaction_id)
            .bind(&material_id)
            .bind(receipt_line.receipt_qty)
            .bind(&to_bin)
            .bind(&receipt_line.batch_number)
            .bind(&operator)
            .bind(&command.po_id)
            .bind(&command.remark)
            .bind(posting_date)
            .bind(unit_price)
            .execute(&mut *tx)
            .await
            .map_err(map_inventory_db_error)?;

            sqlx::query(
                r#"
                UPDATE wms.wms_purchase_orders_d
                SET received_qty = received_qty + $3,
                    line_status = CASE
                        WHEN received_qty + $3 >= ordered_qty THEN '完成'
                        ELSE '部分到货'
                    END
                WHERE po_id = $1
                  AND line_no = $2
                "#,
            )
            .bind(&command.po_id)
            .bind(receipt_line.line_no)
            .bind(receipt_line.receipt_qty)
            .execute(&mut *tx)
            .await?;

            transaction_results.push(json!({
                "transaction_id": transaction_id,
                "movement_type": "101",
                "material_id": material_id,
                "quantity": receipt_line.receipt_qty,
                "batch_number": receipt_line.batch_number,
                "to_bin": to_bin
            }));
        }

        let new_status = Self::refresh_po_status(&mut tx, &command.po_id).await?;
        Self::refresh_po_total(&mut tx, &command.po_id).await?;

        tx.commit().await?;

        Ok(json!({
            "po_id": command.po_id,
            "status": new_status,
            "transactions": transaction_results,
            "reports_stale": true
        }))
    }

    async fn close_order(&self, po_id: String, operator: String) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_purchase_orders_h
            SET status = '取消',
                notes = COALESCE(notes, '') || E'\nClosed by ' || $2 || ' at ' || NOW(),
                updated_at = NOW()
            WHERE po_id = $1
              AND status <> '完成'
              AND status <> '取消'
            RETURNING po_id, status
            "#,
        )
        .bind(&po_id)
        .bind(&operator)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = result else {
            return Err(AppError::Validation(
                "采购订单不存在，或已经完成/取消，不能关闭".to_string(),
            ));
        };

        Ok(json!({
            "po_id": row.get::<String, _>("po_id"),
            "status": row.get::<String, _>("status")
        }))
    }
}
