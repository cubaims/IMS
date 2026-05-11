use async_trait::async_trait;
use cuba_shared::{AppError, AppResult, map_inventory_db_error, map_purchase_db_error};
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgRow};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderClosed,
    PurchaseOrderCreated, PurchaseOrderDetail, PurchaseOrderLineDetail, PurchaseOrderQuery,
    PurchaseOrderRepository, PurchaseOrderSummary, PurchaseReceiptPosted,
    PurchaseReceiptTransaction,
};
use crate::domain::{
    PurchaseDomainError, PurchaseLineStatus, PurchaseOrder, PurchaseOrderLine, PurchaseOrderStatus,
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

    async fn ensure_active_supplier(
        tx: &mut Transaction<'_, Postgres>,
        supplier_id: &str,
    ) -> AppResult<()> {
        let row = sqlx::query(
            r#"
            SELECT is_active
            FROM mdm.mdm_suppliers
            WHERE supplier_id = $1
            "#,
        )
        .bind(supplier_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_purchase_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "SUPPLIER_NOT_FOUND",
                format!("供应商不存在: {supplier_id}"),
            ));
        };

        if !row.get::<bool, _>("is_active") {
            return Err(AppError::business(
                "SUPPLIER_INACTIVE",
                format!("供应商已停用: {supplier_id}"),
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
        .map_err(map_purchase_db_error)?;

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
        .map_err(map_purchase_db_error)?;

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

    async fn po_exists_for_update(
        tx: &mut Transaction<'_, Postgres>,
        po_id: &str,
    ) -> AppResult<PurchaseOrderStatus> {
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
        .await
        .map_err(map_purchase_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("采购订单不存在: {po_id}")));
        };

        let status_text: String = row.get("status");
        PurchaseOrderStatus::from_db_text(&status_text).map_err(Self::map_domain_error)
    }

    fn map_domain_error(error: PurchaseDomainError) -> AppError {
        match error {
            PurchaseDomainError::PurchaseOrderLineNotFound => {
                AppError::NotFound("采购订单行不存在".to_string())
            }
            PurchaseDomainError::InvalidLineNo | PurchaseDomainError::InvalidQuantity => {
                AppError::Validation(error.to_string())
            }
            PurchaseDomainError::ReceiptQuantityExceeded => {
                AppError::business("PO_RECEIPT_QTY_EXCEEDED", "收货数量超过未收数量")
            }
            PurchaseDomainError::EmptyPurchaseOrder => {
                AppError::Validation("采购订单至少需要一行有效明细".to_string())
            }
            PurchaseDomainError::DuplicatedLineNo => {
                AppError::business("PO_LINE_DUPLICATED", "采购订单行号重复")
            }
            _ => AppError::business("PO_STATUS_INVALID", error.to_string()),
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
        .await
        .map_err(map_purchase_db_error)?;

        Ok(())
    }

    async fn persist_po_status(
        tx: &mut Transaction<'_, Postgres>,
        po_id: &str,
        status: PurchaseOrderStatus,
    ) -> AppResult<String> {
        let new_status = status.as_db_text();

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
        .await
        .map_err(map_purchase_db_error)?;

        Ok(new_status.to_string())
    }

    fn purchase_line_from_row(row: &PgRow) -> AppResult<PurchaseOrderLine> {
        let line_status: String = row.get("line_status");

        Ok(PurchaseOrderLine {
            line_no: row.get("line_no"),
            material_id: row.get("material_id"),
            ordered_qty: row.get("ordered_qty"),
            received_qty: row.get("received_qty"),
            unit_price: row.get("unit_price"),
            expected_bin: row.get("expected_bin"),
            line_status: PurchaseLineStatus::from_db_text(&line_status)
                .map_err(Self::map_domain_error)?,
        })
    }

    async fn lock_purchase_lines_for_update(
        tx: &mut Transaction<'_, Postgres>,
        po_id: &str,
    ) -> AppResult<Vec<PurchaseOrderLine>> {
        let rows = sqlx::query(
            r#"
            SELECT
                line_no,
                material_id,
                ordered_qty,
                received_qty,
                unit_price,
                expected_bin,
                line_status
            FROM wms.wms_purchase_orders_d
            WHERE po_id = $1
            ORDER BY line_no
            FOR UPDATE
            "#,
        )
        .bind(po_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_purchase_db_error)?;

        rows.iter()
            .map(Self::purchase_line_from_row)
            .collect::<AppResult<Vec<_>>>()
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
        .await
        .map_err(map_purchase_db_error)?;

        Ok(())
    }

    fn purchase_order_summary_from_row(row: &PgRow) -> PurchaseOrderSummary {
        PurchaseOrderSummary {
            po_id: row.get("po_id"),
            supplier_id: row.get("supplier_id"),
            supplier_name: row.get("supplier_name"),
            po_date: row.get("po_date"),
            expected_date: row.get("expected_date"),
            total_amount: row.get("total_amount"),
            currency: row.get("currency"),
            status: row.get("status"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn purchase_order_line_detail_from_row(row: &PgRow) -> PurchaseOrderLineDetail {
        PurchaseOrderLineDetail {
            id: row.get("id"),
            po_id: row.get("po_id"),
            line_no: row.get("line_no"),
            material_id: row.get("material_id"),
            ordered_qty: row.get("ordered_qty"),
            received_qty: row.get("received_qty"),
            open_qty: row.get("open_qty"),
            unit_price: row.get("unit_price"),
            line_amount: row.get("line_amount"),
            expected_bin: row.get("expected_bin"),
            line_status: row.get("line_status"),
            created_at: row.get("created_at"),
        }
    }

    async fn order_detail_by_id(pool: &PgPool, po_id: &str) -> AppResult<PurchaseOrderDetail> {
        let row = sqlx::query(
            r#"
            SELECT
                po_id,
                supplier_id,
                po_date,
                expected_date,
                total_amount,
                currency,
                status,
                created_by,
                approved_by,
                approved_at,
                notes,
                created_at,
                updated_at
            FROM wms.wms_purchase_orders_h
            WHERE po_id = $1
            "#,
        )
        .bind(po_id)
        .fetch_optional(pool)
        .await
        .map_err(map_purchase_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("采购订单不存在: {po_id}")));
        };

        let line_rows = sqlx::query(
            r#"
            SELECT
                id,
                po_id,
                line_no,
                material_id,
                ordered_qty,
                received_qty,
                open_qty,
                unit_price,
                line_amount,
                expected_bin,
                line_status,
                created_at
            FROM wms.wms_purchase_orders_d
            WHERE po_id = $1
            ORDER BY line_no
            "#,
        )
        .bind(po_id)
        .fetch_all(pool)
        .await
        .map_err(map_purchase_db_error)?;

        Ok(PurchaseOrderDetail {
            po_id: row.get("po_id"),
            supplier_id: row.get("supplier_id"),
            po_date: row.get("po_date"),
            expected_date: row.get("expected_date"),
            total_amount: row.get("total_amount"),
            currency: row.get("currency"),
            status: row.get("status"),
            created_by: row.get("created_by"),
            approved_by: row.get("approved_by"),
            approved_at: row.get("approved_at"),
            notes: row.get("notes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            lines: line_rows
                .iter()
                .map(Self::purchase_order_line_detail_from_row)
                .collect(),
        })
    }
}

#[async_trait]
impl PurchaseOrderRepository for PostgresPurchaseOrderRepository {
    async fn create_order(
        &self,
        command: CreatePurchaseOrderCommand,
        operator: String,
    ) -> AppResult<PurchaseOrderCreated> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("采购订单至少需要一行明细".to_string()));
        }

        let mut tx = self.pool.begin().await.map_err(map_purchase_db_error)?;
        let po_id = Self::next_po_id();

        Self::ensure_active_supplier(&mut tx, &command.supplier_id).await?;

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
        .await
        .map_err(map_purchase_db_error)?;

        for line in command.lines {
            Self::ensure_active_material(&mut tx, &line.material_id).await?;
            if let Some(expected_bin) = &line.expected_bin {
                Self::ensure_active_bin(&mut tx, expected_bin).await?;
            }

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
            .await
            .map_err(map_purchase_db_error)?;
        }

        Self::refresh_po_total(&mut tx, &po_id).await?;
        tx.commit().await.map_err(map_purchase_db_error)?;

        Ok(PurchaseOrderCreated {
            po_id,
            status: "已审批".to_string(),
            reports_stale: false,
        })
    }

    async fn list_orders(&self, query: PurchaseOrderQuery) -> AppResult<Vec<PurchaseOrderSummary>> {
        let rows = sqlx::query(
            r#"
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
            "#,
        )
        .bind(&query.supplier_id)
        .bind(&query.status)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(&query.material_id)
        .bind(query.limit())
        .bind(query.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(map_purchase_db_error)?;

        Ok(rows
            .iter()
            .map(Self::purchase_order_summary_from_row)
            .collect())
    }

    async fn get_order(&self, po_id: String) -> AppResult<PurchaseOrderDetail> {
        Self::order_detail_by_id(&self.pool, &po_id).await
    }

    async fn post_receipt(
        &self,
        command: PostPurchaseReceiptCommand,
        operator: String,
    ) -> AppResult<PurchaseReceiptPosted> {
        if command.lines.is_empty() {
            return Err(AppError::Validation("收货明细不能为空".to_string()));
        }

        let posting_date: OffsetDateTime =
            command.posting_date.unwrap_or_else(OffsetDateTime::now_utc);
        let mut tx = self.pool.begin().await.map_err(map_purchase_db_error)?;

        sqlx::query("SET LOCAL lock_timeout = '5s'")
            .execute(&mut *tx)
            .await
            .map_err(map_purchase_db_error)?;

        let status = Self::po_exists_for_update(&mut tx, &command.po_id).await?;
        status
            .ensure_can_receive()
            .map_err(Self::map_domain_error)?;
        let mut order_lines = Self::lock_purchase_lines_for_update(&mut tx, &command.po_id).await?;

        let mut transactions = Vec::new();

        for receipt_line in command.lines {
            let order_line = order_lines
                .iter_mut()
                .find(|line| line.line_no == receipt_line.line_no)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "采购订单行不存在: po_id={}, line_no={}",
                        command.po_id, receipt_line.line_no
                    ))
                })?;

            let material_id = order_line.material_id.clone();
            let unit_price = order_line.unit_price;
            let expected_bin = order_line.expected_bin.clone();
            order_line
                .receive(receipt_line.receipt_qty)
                .map_err(Self::map_domain_error)?;

            let to_bin = receipt_line
                .to_bin
                .clone()
                .or(expected_bin)
                .ok_or_else(|| AppError::Validation("收货必须指定目标货位".to_string()))?;

            Self::ensure_active_material(&mut tx, &material_id).await?;
            Self::ensure_active_bin(&mut tx, &to_bin).await?;

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
                SET received_qty = $3,
                    line_status = $4
                WHERE po_id = $1
                  AND line_no = $2
                "#,
            )
            .bind(&command.po_id)
            .bind(receipt_line.line_no)
            .bind(order_line.received_qty)
            .bind(order_line.line_status.as_db_text())
            .execute(&mut *tx)
            .await
            .map_err(map_purchase_db_error)?;

            transactions.push(PurchaseReceiptTransaction {
                transaction_id,
                movement_type: "101".to_string(),
                material_id,
                quantity: receipt_line.receipt_qty,
                batch_number: Some(receipt_line.batch_number),
                to_bin: Some(to_bin),
            });
        }

        let new_status =
            PurchaseOrder::status_from_lines(&order_lines).map_err(Self::map_domain_error)?;
        let new_status = Self::persist_po_status(&mut tx, &command.po_id, new_status).await?;
        Self::refresh_po_total(&mut tx, &command.po_id).await?;

        tx.commit().await.map_err(map_purchase_db_error)?;

        Ok(PurchaseReceiptPosted {
            po_id: command.po_id,
            status: new_status,
            transactions,
            reports_stale: true,
        })
    }

    async fn close_order(&self, po_id: String, operator: String) -> AppResult<PurchaseOrderClosed> {
        let mut tx = self.pool.begin().await.map_err(map_purchase_db_error)?;
        let status = Self::po_exists_for_update(&mut tx, &po_id).await?;
        status.ensure_can_close().map_err(Self::map_domain_error)?;

        let lines = Self::lock_purchase_lines_for_update(&mut tx, &po_id).await?;
        for line in lines {
            if line.line_status != PurchaseLineStatus::Completed {
                sqlx::query(
                    r#"
                    UPDATE wms.wms_purchase_orders_d
                    SET line_status = $3
                    WHERE po_id = $1
                      AND line_no = $2
                    "#,
                )
                .bind(&po_id)
                .bind(line.line_no)
                .bind(PurchaseLineStatus::Cancelled.as_db_text())
                .execute(&mut *tx)
                .await
                .map_err(map_purchase_db_error)?;
            }
        }

        let result = sqlx::query(
            r#"
            UPDATE wms.wms_purchase_orders_h
            SET status = $3,
                notes = COALESCE(notes, '') || E'\nClosed by ' || $2 || ' at ' || NOW(),
                updated_at = NOW()
            WHERE po_id = $1
            RETURNING po_id, status
            "#,
        )
        .bind(&po_id)
        .bind(&operator)
        .bind(PurchaseOrderStatus::Closed.as_db_text())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_purchase_db_error)?;

        let Some(row) = result else {
            return Err(AppError::Validation(
                "采购订单不存在，或已经完成/取消，不能关闭".to_string(),
            ));
        };

        tx.commit().await.map_err(map_purchase_db_error)?;

        Ok(PurchaseOrderClosed {
            po_id: row.get("po_id"),
            status: row.get("status"),
        })
    }
}
