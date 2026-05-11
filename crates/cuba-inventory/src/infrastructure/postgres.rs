use async_trait::async_trait;
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use cuba_shared::{AppError, AppResult, map_inventory_db_error};

use crate::{
    application::{
        BatchHistoryQuery, BatchQuery, BatchRepository, CurrentStockQuery, InventoryRepository,
        InventoryTransactionQuery, MapHistoryQuery, MapHistoryRepository, PickBatchFefoCommand,
        PostInventoryCommand, common::Page,
    },
    domain::{
        Batch, BatchHistory, BinStock, CurrentStock, InventoryPostingResult, InventoryTransaction,
        MapHistory, MaterialId, MovementType, TransactionId,
    },
};

#[derive(Clone)]
pub struct PostgresInventoryRepository {
    pool: PgPool,
}

impl PostgresInventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn next_transaction_id(prefix: &str) -> String {
        let raw = format!("{}-{}", prefix, Uuid::now_v7());
        raw.chars().take(30).collect()
    }

    fn db_error(err: sqlx::Error) -> AppError {
        map_inventory_db_error(err)
    }

    fn parse_transaction_row(row: sqlx::postgres::PgRow) -> AppResult<InventoryTransaction> {
        let transaction_id: String = row.try_get("transaction_id").map_err(Self::db_error)?;
        let material_id: String = row.try_get("material_id").map_err(Self::db_error)?;
        let movement_type: String = row.try_get("movement_type").map_err(Self::db_error)?;
        let quantity: Decimal = row.try_get("quantity").map_err(Self::db_error)?;
        let transaction_date: OffsetDateTime =
            row.try_get("transaction_date").map_err(Self::db_error)?;

        let movement_type = movement_type.parse::<MovementType>().map_err(|err| {
            AppError::Validation(format!("invalid movement type in database: {}", err))
        })?;

        Ok(InventoryTransaction {
            transaction_id: TransactionId::new(transaction_id)
                .map_err(|err| AppError::Validation(err.to_string()))?,
            material_id: MaterialId::new(material_id)
                .map_err(|err| AppError::Validation(err.to_string()))?,
            movement_type,
            quantity,
            from_bin: row.try_get("from_bin").map_err(Self::db_error)?,
            to_bin: row.try_get("to_bin").map_err(Self::db_error)?,
            batch_number: row.try_get("batch_number").map_err(Self::db_error)?,
            serial_number: row.try_get("serial_number").map_err(Self::db_error)?,
            reference_doc: row.try_get("reference_doc").map_err(Self::db_error)?,
            operator: row.try_get("operator").map_err(Self::db_error)?,
            transaction_date,
            remark: row.try_get("remarks").map_err(Self::db_error)?,
        })
    }

    fn parse_current_stock_row(row: sqlx::postgres::PgRow) -> AppResult<CurrentStock> {
        Ok(CurrentStock {
            material_id: row.try_get("material_id").map_err(Self::db_error)?,
            material_name: row.try_get("material_name").map_err(Self::db_error)?,
            bin_code: row.try_get("bin_code").map_err(Self::db_error)?,
            zone: row.try_get("zone").map_err(Self::db_error)?,
            batch_number: row.try_get("batch_number").map_err(Self::db_error)?,
            quality_status: row.try_get("quality_status").map_err(Self::db_error)?,
            qty: row.try_get("qty").map_err(Self::db_error)?,
            serial_count: row.try_get("serial_count").map_err(Self::db_error)?,
            last_transaction_at: row.try_get("last_transaction_at").map_err(Self::db_error)?,
        })
    }

    fn parse_bin_stock_row(row: sqlx::postgres::PgRow) -> AppResult<BinStock> {
        Ok(BinStock {
            material_id: row.try_get("material_id").map_err(Self::db_error)?,
            bin_code: row.try_get("bin_code").map_err(Self::db_error)?,
            batch_number: row.try_get("batch_number").map_err(Self::db_error)?,
            quality_status: row.try_get("quality_status").map_err(Self::db_error)?,
            qty: row.try_get("qty").map_err(Self::db_error)?,
            updated_at: row.try_get("updated_at").map_err(Self::db_error)?,
        })
    }

    fn parse_batch_row(row: sqlx::postgres::PgRow) -> AppResult<Batch> {
        Ok(Batch {
            batch_number: row.try_get("batch_number").map_err(Self::db_error)?,
            material_id: row.try_get("material_id").map_err(Self::db_error)?,
            production_date: row.try_get("production_date").map_err(Self::db_error)?,
            expiry_date: row.try_get("expiry_date").map_err(Self::db_error)?,
            quality_grade: row.try_get("quality_grade").map_err(Self::db_error)?,
            current_stock: row.try_get("current_stock").map_err(Self::db_error)?,
            current_bin: row.try_get("current_bin").map_err(Self::db_error)?,
            quality_status: row.try_get("quality_status").map_err(Self::db_error)?,
            created_at: row.try_get("created_at").map_err(Self::db_error)?,
            updated_at: row.try_get("updated_at").map_err(Self::db_error)?,
        })
    }

    fn parse_batch_history_row(row: sqlx::postgres::PgRow) -> AppResult<BatchHistory> {
        Ok(BatchHistory {
            history_id: row.try_get("history_id").map_err(Self::db_error)?,
            batch_number: row.try_get("batch_number").map_err(Self::db_error)?,
            material_id: row.try_get("material_id").map_err(Self::db_error)?,
            event_type: row.try_get("event_type").map_err(Self::db_error)?,
            old_quality_status: row.try_get("old_quality_status").map_err(Self::db_error)?,
            new_quality_status: row.try_get("new_quality_status").map_err(Self::db_error)?,
            old_bin: row.try_get("old_bin").map_err(Self::db_error)?,
            new_bin: row.try_get("new_bin").map_err(Self::db_error)?,
            old_stock: row.try_get("old_stock").map_err(Self::db_error)?,
            new_stock: row.try_get("new_stock").map_err(Self::db_error)?,
            transaction_id: row.try_get("transaction_id").map_err(Self::db_error)?,
            changed_by: row.try_get("changed_by").map_err(Self::db_error)?,
            changed_at: row.try_get("changed_at").map_err(Self::db_error)?,
            remarks: row.try_get("remarks").map_err(Self::db_error)?,
        })
    }

    fn parse_map_history_row(row: sqlx::postgres::PgRow) -> AppResult<MapHistory> {
        Ok(MapHistory {
            history_id: row.try_get("history_id").map_err(Self::db_error)?,
            material_id: row.try_get("material_id").map_err(Self::db_error)?,
            old_map_price: row.try_get("old_map_price").map_err(Self::db_error)?,
            new_map_price: row.try_get("new_map_price").map_err(Self::db_error)?,
            old_stock_qty: row.try_get("old_stock_qty").map_err(Self::db_error)?,
            new_stock_qty: row.try_get("new_stock_qty").map_err(Self::db_error)?,
            incoming_qty: row.try_get("incoming_qty").map_err(Self::db_error)?,
            incoming_unit_price: row.try_get("incoming_unit_price").map_err(Self::db_error)?,
            transaction_id: row.try_get("transaction_id").map_err(Self::db_error)?,
            calculation_formula: row.try_get("calculation_formula").map_err(Self::db_error)?,
            changed_by: row.try_get("changed_by").map_err(Self::db_error)?,
            changed_at: row.try_get("changed_at").map_err(Self::db_error)?,
        })
    }

    fn page_from_rows<T>(
        rows: Vec<sqlx::postgres::PgRow>,
        page_number: u64,
        page_size: u64,
        total: u64,
        parse: fn(sqlx::postgres::PgRow) -> AppResult<T>,
    ) -> AppResult<Page<T>> {
        let items = rows.into_iter().map(parse).collect::<AppResult<Vec<_>>>()?;

        Ok(Page::new(items, page_number, page_size, total))
    }

    fn map_db_error_to_inventory_error(err: sqlx::Error) -> AppError {
        let message = err.to_string();

        if message.contains("库存不足") || message.contains("Insufficient stock") {
            return AppError::business("INSUFFICIENT_STOCK", "库存不足");
        }

        if message.contains("货位容量")
            || message.contains("容量不足")
            || message.contains("capacity")
        {
            return AppError::business("BIN_CAPACITY_EXCEEDED", "目标货位容量不足");
        }

        if message.contains("冻结") {
            return AppError::business("BATCH_FROZEN", "批次已冻结");
        }

        if message.contains("报废") {
            return AppError::business("BATCH_SCRAPPED", "批次已报废");
        }

        if message.contains("物料") && message.contains("不存在") {
            return AppError::business("MATERIAL_NOT_FOUND", "物料不存在");
        }

        if message.contains("货位") && message.contains("不存在") {
            return AppError::business("BIN_NOT_FOUND", "货位不存在");
        }

        if message.contains("批次") && message.contains("不存在") {
            return AppError::business("BATCH_NOT_FOUND", "批次不存在");
        }

        map_inventory_db_error(err)
    }
}

#[async_trait]
impl InventoryRepository for PostgresInventoryRepository {
    async fn post_inventory_transaction(
        &self,
        command: PostInventoryCommand,
        operator: String,
    ) -> AppResult<InventoryPostingResult> {
        let transaction_id = Self::next_transaction_id(&command.movement_type);
        let movement_type = command.movement_type.clone();
        let posting_date = command.posting_date.unwrap_or_else(OffsetDateTime::now_utc);
        let quantity = command.quantity_as_i32().map_err(AppError::Validation)?;
        let quality_status = command
            .quality_status
            .clone()
            .unwrap_or_else(|| "合格".to_string());

        sqlx::query(
            r#"
            SELECT wms.post_inventory_transaction(
                $1,
                $2::wms.movement_type,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10::mdm.quality_status,
                $11,
                $12,
                $13,
                $14
            )
            "#,
        )
        .bind(&transaction_id)
        .bind(&movement_type)
        .bind(&command.material_id)
        .bind(quantity)
        .bind(&command.from_bin)
        .bind(&command.to_bin)
        .bind(&command.batch_number)
        .bind(&command.serial_number)
        .bind(&operator)
        .bind(&quality_status)
        .bind(&command.reference_doc)
        .bind(&command.remark)
        .bind(posting_date)
        .bind(command.unit_price)
        .execute(&self.pool)
        .await
        .map_err(Self::map_db_error_to_inventory_error)?;

        Ok(InventoryPostingResult {
            transaction_id,
            material_id: command.material_id,
            movement_type: command.movement_type,
            quantity: command.quantity,
            from_bin: command.from_bin,
            to_bin: command.to_bin,
            batch_number: command.batch_number,
            reference_doc: command.reference_doc,
            map_updated: command.unit_price.is_some(),
        })
    }

    async fn list_current_stock(&self, query: CurrentStockQuery) -> AppResult<Page<CurrentStock>> {
        let page = query.page();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM (
                SELECT 1
                FROM wms.wms_bin_stock bs
                JOIN mdm.mdm_materials m ON m.material_id = bs.material_id
                JOIN mdm.mdm_storage_bins b ON b.bin_code = bs.bin_code
                WHERE ($1::TEXT IS NULL OR bs.material_id = $1)
                  AND ($2::TEXT IS NULL OR bs.bin_code = $2)
                  AND ($3::TEXT IS NULL OR bs.batch_number = $3)
                  AND ($4::TEXT IS NULL OR b.zone = $4)
                  AND ($5::TEXT IS NULL OR bs.quality_status::TEXT = $5)
                  AND ($6::BOOLEAN IS NULL OR $6 = FALSE OR bs.qty > 0)
                  AND ($7::BOOLEAN IS NULL OR $7 = FALSE OR m.current_stock < m.safety_stock)
                GROUP BY bs.material_id, m.material_name, bs.bin_code, b.zone, bs.batch_number,
                         bs.quality_status, bs.qty
            ) counted
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.bin_code)
        .bind(&query.batch_number)
        .bind(&query.zone)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .bind(query.only_low_stock)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                bs.material_id,
                m.material_name,
                bs.bin_code,
                b.zone,
                bs.batch_number,
            bs.quality_status::TEXT as quality_status,
            bs.qty::NUMERIC AS qty,
                COUNT(DISTINCT sn.serial_number)::INTEGER AS serial_count,
                lt.last_transaction_at
            FROM wms.wms_bin_stock bs
            JOIN mdm.mdm_materials m ON m.material_id = bs.material_id
            JOIN mdm.mdm_storage_bins b ON b.bin_code = bs.bin_code
            LEFT JOIN wms.wms_batches ba ON ba.batch_number = bs.batch_number
            LEFT JOIN wms.wms_serial_numbers sn ON sn.material_id = bs.material_id
                AND sn.batch_number = bs.batch_number
                AND sn.current_bin = bs.bin_code
            LEFT JOIN (
                SELECT material_id, batch_number, bin_code, MAX(transaction_date) AS last_transaction_at
                FROM (
                    SELECT material_id, batch_number, to_bin AS bin_code, transaction_date
                    FROM wms.wms_transactions
                    WHERE to_bin IS NOT NULL
                    UNION ALL
                    SELECT material_id, batch_number, from_bin AS bin_code, transaction_date
                    FROM wms.wms_transactions
                    WHERE from_bin IS NOT NULL
                ) x
                GROUP BY material_id, batch_number, bin_code
            ) lt ON lt.material_id = bs.material_id
                AND lt.batch_number = bs.batch_number
                AND lt.bin_code = bs.bin_code
            WHERE ($1::TEXT IS NULL OR bs.material_id = $1)
              AND ($2::TEXT IS NULL OR bs.bin_code = $2)
              AND ($3::TEXT IS NULL OR bs.batch_number = $3)
              AND ($4::TEXT IS NULL OR b.zone = $4)
              AND ($5::TEXT IS NULL OR bs.quality_status::TEXT = $5)
              AND ($6::BOOLEAN IS NULL OR $6 = FALSE OR bs.qty > 0)
              AND ($7::BOOLEAN IS NULL OR $7 = FALSE OR m.current_stock < m.safety_stock)
            GROUP BY bs.material_id, m.material_name, bs.bin_code, b.zone, bs.batch_number,
                     bs.quality_status, bs.qty, lt.last_transaction_at
            ORDER BY bs.material_id, bs.bin_code, bs.batch_number
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.bin_code)
        .bind(&query.batch_number)
        .bind(&query.zone)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .bind(query.only_low_stock)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        Self::page_from_rows(
            rows,
            page.page_number(),
            page.page_size_value(),
            total.max(0) as u64,
            Self::parse_current_stock_row,
        )
    }

    async fn list_bin_stock(&self, query: CurrentStockQuery) -> AppResult<Page<BinStock>> {
        let page = query.page();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_bin_stock
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR bin_code = $2)
              AND ($3::TEXT IS NULL OR batch_number = $3)
              AND ($4::TEXT IS NULL OR quality_status::TEXT = $4)
              AND ($5::BOOLEAN IS NULL OR $5 = FALSE OR qty > 0)
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.bin_code)
        .bind(&query.batch_number)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                material_id,
                bin_code,
                batch_number,
                quality_status::TEXT as quality_status,
                qty::NUMERIC AS qty,
                updated_at
            FROM wms.wms_bin_stock
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR bin_code = $2)
              AND ($3::TEXT IS NULL OR batch_number = $3)
              AND ($4::TEXT IS NULL OR quality_status::TEXT = $4)
              AND ($5::BOOLEAN IS NULL OR $5 = FALSE OR qty > 0)
            ORDER BY material_id, bin_code, batch_number
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.bin_code)
        .bind(&query.batch_number)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        Self::page_from_rows(
            rows,
            page.page_number(),
            page.page_size_value(),
            total.max(0) as u64,
            Self::parse_bin_stock_row,
        )
    }

    async fn stock_by_zone(&self, query: CurrentStockQuery) -> AppResult<Value> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(row_to_json(x)), '[]'::json) AS data
            FROM (
                SELECT
                    material_id,
                    material_name,
                    material_type::TEXT AS material_type,
                    qty_rm,
                    qty_sf,
                    qty_fg,
                    qty_prod,
                    total_qty,
                    safety_stock,
                    map_price,
                    total_value,
                    status
                FROM rpt.rpt_stock_by_zone
                WHERE ($1::TEXT IS NULL OR material_id = $1)
                  AND ($2::BOOLEAN IS NULL OR $2 = FALSE OR total_qty < safety_stock)
                ORDER BY material_id
                LIMIT $3 OFFSET $4
            ) x
            "#,
        )
        .bind(&query.material_id)
        .bind(query.only_low_stock)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        rows.try_get("data").map_err(Self::db_error)
    }

    async fn bin_summary(&self, query: CurrentStockQuery) -> AppResult<Value> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(row_to_json(x)), '[]'::json) AS data
            FROM (
                SELECT
                    bin_code,
                    zone,
                    bin_type,
                    capacity,
                    current_qty,
                    material_count,
                    batch_count,
                    utilization_pct,
                    last_movement_at,
                    status
                FROM rpt.rpt_bin_stock_summary
                WHERE ($1::TEXT IS NULL OR bin_code = $1)
                  AND ($2::TEXT IS NULL OR zone = $2)
                ORDER BY zone, bin_code
                LIMIT $3 OFFSET $4
            ) x
            "#,
        )
        .bind(&query.bin_code)
        .bind(&query.zone)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        rows.try_get("data").map_err(Self::db_error)
    }

    async fn batch_summary(&self, query: CurrentStockQuery) -> AppResult<Value> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(row_to_json(x)), '[]'::json) AS data
            FROM (
                SELECT
                    batch_number,
                    material_id,
                    material_name,
                    production_date,
                    expiry_date,
                    days_to_expiry,
                    supplier_batch,
                    quality_grade,
                    quality_status::TEXT AS quality_status,
                    current_stock,
                    bins,
                    age_days,
                    alert_level
                FROM rpt.rpt_batch_stock_summary
                WHERE ($1::TEXT IS NULL OR material_id = $1)
                  AND ($2::TEXT IS NULL OR batch_number = $2)
                  AND ($3::TEXT IS NULL OR quality_status::TEXT = $3)
                  AND ($4::BOOLEAN IS NULL OR $4 = FALSE OR current_stock > 0)
                ORDER BY material_id, expiry_date NULLS LAST, batch_number
                LIMIT $5 OFFSET $6
            ) x
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.batch_number)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        rows.try_get("data").map_err(Self::db_error)
    }

    async fn list_transactions(
        &self,
        query: InventoryTransactionQuery,
    ) -> AppResult<Page<InventoryTransaction>> {
        let page = query.page();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_transactions
            WHERE ($1::TEXT IS NULL OR transaction_id = $1)
              AND ($2::TEXT IS NULL OR material_id = $2)
              AND ($3::TEXT IS NULL OR movement_type::TEXT = $3)
              AND ($4::TEXT IS NULL OR batch_number = $4)
              AND ($5::TEXT IS NULL OR from_bin = $5)
              AND ($6::TEXT IS NULL OR to_bin = $6)
              AND ($7::TEXT IS NULL OR reference_doc = $7)
              AND ($8::TEXT IS NULL OR operator = $8)
              AND ($9::TIMESTAMPTZ IS NULL OR transaction_date >= $9)
              AND ($10::TIMESTAMPTZ IS NULL OR transaction_date <= $10)
            "#,
        )
        .bind(&query.transaction_id)
        .bind(&query.material_id)
        .bind(&query.movement_type)
        .bind(&query.batch_number)
        .bind(&query.from_bin)
        .bind(&query.to_bin)
        .bind(&query.reference_doc)
        .bind(&query.operator)
        .bind(query.date_from)
        .bind(query.date_to)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                transaction_id,
                movement_type::TEXT AS movement_type,
                material_id,
                quantity::NUMERIC AS quantity,
                from_bin,
                to_bin,
                batch_number,
                serial_number,
                reference_doc,
                operator,
                transaction_date,
                notes AS remarks
            FROM wms.wms_transactions
            WHERE ($1::TEXT IS NULL OR transaction_id = $1)
              AND ($2::TEXT IS NULL OR material_id = $2)
              AND ($3::TEXT IS NULL OR movement_type::TEXT = $3)
              AND ($4::TEXT IS NULL OR batch_number = $4)
              AND ($5::TEXT IS NULL OR from_bin = $5)
              AND ($6::TEXT IS NULL OR to_bin = $6)
              AND ($7::TEXT IS NULL OR reference_doc = $7)
              AND ($8::TEXT IS NULL OR operator = $8)
              AND ($9::TIMESTAMPTZ IS NULL OR transaction_date >= $9)
              AND ($10::TIMESTAMPTZ IS NULL OR transaction_date <= $10)
            ORDER BY transaction_date DESC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(&query.transaction_id)
        .bind(&query.material_id)
        .bind(&query.movement_type)
        .bind(&query.batch_number)
        .bind(&query.from_bin)
        .bind(&query.to_bin)
        .bind(&query.reference_doc)
        .bind(&query.operator)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        Self::page_from_rows(
            rows,
            page.page_number(),
            page.page_size_value(),
            total.max(0) as u64,
            Self::parse_transaction_row,
        )
    }

    async fn get_transaction(
        &self,
        transaction_id: String,
    ) -> AppResult<Option<InventoryTransaction>> {
        let row = sqlx::query(
            r#"
            SELECT
                transaction_id,
                movement_type::TEXT AS movement_type,
                material_id,
                quantity::NUMERIC AS quantity,
                from_bin,
                to_bin,
                batch_number,
                serial_number,
                reference_doc,
                operator,
                transaction_date,
                notes AS remarks
            FROM wms.wms_transactions
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db_error)?;

        row.map(Self::parse_transaction_row).transpose()
    }
}

#[async_trait]
impl BatchRepository for PostgresInventoryRepository {
    async fn list_batches(&self, query: BatchQuery) -> AppResult<Page<Batch>> {
        let page = query.page();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_batches
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR batch_number = $2)
              AND ($3::TEXT IS NULL OR quality_status::TEXT = $3)
              AND ($4::BOOLEAN IS NULL OR $4 = FALSE OR current_stock > 0)
              AND ($5::BOOLEAN IS NULL OR $5 = FALSE OR expiry_date <= CURRENT_DATE + INTERVAL '30 days')
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.batch_number)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .bind(query.only_expiring)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                batch_number,
                material_id,
                production_date,
                expiry_date,
                quality_grade,
                current_stock::NUMERIC AS current_stock,
                current_bin,
                quality_status::TEXT as quality_status,
                created_at,
                updated_at
            FROM wms.wms_batches
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR batch_number = $2)
              AND ($3::TEXT IS NULL OR quality_status::TEXT = $3)
              AND ($4::BOOLEAN IS NULL OR $4 = FALSE OR current_stock > 0)
              AND ($5::BOOLEAN IS NULL OR $5 = FALSE OR expiry_date <= CURRENT_DATE + INTERVAL '30 days')
            ORDER BY material_id, expiry_date NULLS LAST, batch_number
            LIMIT $6 OFFSET $7
            "#,
        )
            .bind(&query.material_id)
            .bind(&query.batch_number)
            .bind(&query.quality_status)
            .bind(query.only_available)
            .bind(query.only_expiring)
            .bind(page.limit())
            .bind(page.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(Self::db_error)?;

        Self::page_from_rows(
            rows,
            page.page_number(),
            page.page_size_value(),
            total.max(0) as u64,
            Self::parse_batch_row,
        )
    }

    async fn get_batch(&self, batch_number: String) -> AppResult<Option<Batch>> {
        let row = sqlx::query(
            r#"
            SELECT
                batch_number,
                material_id,
                production_date,
                expiry_date,
                quality_grade,
                current_stock::NUMERIC AS current_stock,
                current_bin,
                quality_status::TEXT as quality_status,
                created_at,
                updated_at
            FROM wms.wms_batches
            WHERE batch_number = $1
            "#,
        )
        .bind(batch_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::db_error)?;

        row.map(Self::parse_batch_row).transpose()
    }

    async fn list_batch_history(
        &self,
        batch_number: String,
        query: BatchHistoryQuery,
    ) -> AppResult<Page<BatchHistory>> {
        let page = query.page();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_batch_history h
            WHERE h.batch_number = $1
              AND ($2::TEXT IS NULL OR h.change_reason = $2)
              AND ($3::TEXT IS NULL OR h.changed_by = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR h.changed_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR h.changed_at <= $5)
            "#,
        )
        .bind(&batch_number)
        .bind(&query.event_type)
        .bind(&query.operator)
        .bind(query.date_from)
        .bind(query.date_to)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                h.id AS history_id,
                h.batch_number,
                b.material_id,
                COALESCE(h.change_reason, 'BATCH_CHANGE') AS event_type,
                h.old_quality_status::TEXT as old_quality_status,
                h.new_quality_status::TEXT as new_quality_status,
                h.old_bin,
                h.new_bin,
                h.old_stock::NUMERIC AS old_stock,
                h.new_stock::NUMERIC AS new_stock,
                h.transaction_id,
                h.changed_by,
                h.changed_at,
                h.change_reason AS remarks
            FROM wms.wms_batch_history h
            JOIN wms.wms_batches b ON b.batch_number = h.batch_number
            WHERE h.batch_number = $1
              AND ($2::TEXT IS NULL OR h.change_reason = $2)
              AND ($3::TEXT IS NULL OR h.changed_by = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR h.changed_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR h.changed_at <= $5)
            ORDER BY h.changed_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(&batch_number)
        .bind(&query.event_type)
        .bind(&query.operator)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        Self::page_from_rows(
            rows,
            page.page_number(),
            page.page_size_value(),
            total.max(0) as u64,
            Self::parse_batch_history_row,
        )
    }

    async fn pick_batch_fefo(&self, command: PickBatchFefoCommand) -> AppResult<Value> {
        let quantity = command.quantity_as_i32().map_err(AppError::Validation)?;
        let quality_status = command.quality_status.unwrap_or_else(|| "合格".to_string());

        let rows = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(x), '[]'::json) AS data
            FROM wms.fn_pick_batch_fefo(
                $1,
                $2,
                $3,
                $4::mdm.quality_status
            ) x
            "#,
        )
        .bind(command.material_id)
        .bind(quantity)
        .bind(command.from_zone)
        .bind(quality_status)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_db_error_to_inventory_error)?;

        rows.try_get("data").map_err(Self::db_error)
    }
}

#[async_trait]
impl MapHistoryRepository for PostgresInventoryRepository {
    async fn list_map_history(&self, query: MapHistoryQuery) -> AppResult<Page<MapHistory>> {
        let page = query.page();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_map_history
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR transaction_id = $2)
              AND ($3::TIMESTAMPTZ IS NULL OR changed_at >= $3)
              AND ($4::TIMESTAMPTZ IS NULL OR changed_at <= $4)
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.transaction_id)
        .bind(query.date_from)
        .bind(query.date_to)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                id AS history_id,
                material_id,
                COALESCE(old_map_price, 0)::NUMERIC AS old_map_price,
                new_map_price,
                old_stock_qty::NUMERIC AS old_stock_qty,
                new_stock_qty::NUMERIC AS new_stock_qty,
                received_qty::NUMERIC AS incoming_qty,
                COALESCE(received_unit_price, 0)::NUMERIC AS incoming_unit_price,
                transaction_id,
                calculation_formula,
                changed_by,
                changed_at
            FROM wms.wms_map_history
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR transaction_id = $2)
              AND ($3::TIMESTAMPTZ IS NULL OR changed_at >= $3)
              AND ($4::TIMESTAMPTZ IS NULL OR changed_at <= $4)
            ORDER BY changed_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.transaction_id)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        Self::page_from_rows(
            rows,
            page.page_number(),
            page.page_size_value(),
            total.max(0) as u64,
            Self::parse_map_history_row,
        )
    }

    async fn list_material_map_history(
        &self,
        material_id: String,
        mut query: MapHistoryQuery,
    ) -> AppResult<Page<MapHistory>> {
        query.material_id = Some(material_id);
        self.list_map_history(query).await
    }
}
