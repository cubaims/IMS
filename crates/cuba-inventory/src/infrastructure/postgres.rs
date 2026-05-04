use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use cuba_shared::{AppError, AppResult};

use crate::{
    application::{
        BatchHistoryQuery, BatchQuery, BatchRepository, CurrentStockQuery, InventoryRepository,
        InventoryTransactionQuery, MapHistoryQuery, MapHistoryRepository, PickBatchFefoCommand,
        PostInventoryCommand,
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
        AppError::Database(err)
    }

    fn parse_transaction_row(row: sqlx::postgres::PgRow) -> AppResult<InventoryTransaction> {
        let transaction_id: String = row.try_get("transaction_id").map_err(Self::db_error)?;
        let material_id: String = row.try_get("material_id").map_err(Self::db_error)?;
        let movement_type: String = row.try_get("movement_type").map_err(Self::db_error)?;
        let quantity: i32 = row.try_get("quantity").map_err(Self::db_error)?;
        let transaction_date: DateTime<Utc> =
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

    fn map_db_error_to_inventory_error(err: sqlx::Error) -> AppError {
        let message = err.to_string();

        if message.contains("库存不足") || message.contains("Insufficient stock") {
            return AppError::Validation("INSUFFICIENT_STOCK".to_string());
        }

        if message.contains("货位容量") || message.contains("capacity") {
            return AppError::Validation("BIN_CAPACITY_EXCEEDED".to_string());
        }

        if message.contains("冻结") {
            return AppError::Validation("BATCH_FROZEN".to_string());
        }

        if message.contains("报废") {
            return AppError::Validation("BATCH_SCRAPPED".to_string());
        }

        if message.contains("物料") && message.contains("不存在") {
            return AppError::Validation("MATERIAL_NOT_FOUND".to_string());
        }

        if message.contains("货位") && message.contains("不存在") {
            return AppError::Validation("BIN_NOT_FOUND".to_string());
        }

        if message.contains("批次") && message.contains("不存在") {
            return AppError::Validation("BATCH_NOT_FOUND".to_string());
        }

        AppError::Database(err)
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
        let posting_date = command.posting_date.unwrap_or_else(Utc::now);
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
        .bind(command.quantity)
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

    async fn list_current_stock(&self, query: CurrentStockQuery) -> AppResult<Vec<CurrentStock>> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT
                material_id,
                material_name,
                bin_code,
                zone,
                batch_number,
                quality_status::TEXT as quality_status,
                qty,
                serial_count,
                last_transaction_at
            FROM rpt.rpt_current_stock
            WHERE ($1::TEXT IS NULL OR material_id = $1)
              AND ($2::TEXT IS NULL OR bin_code = $2)
              AND ($3::TEXT IS NULL OR batch_number = $3)
              AND ($4::TEXT IS NULL OR zone = $4)
              AND ($5::TEXT IS NULL OR quality_status::TEXT = $5)
              AND ($6::BOOLEAN IS NULL OR $6 = FALSE OR qty > 0)
            ORDER BY material_id, bin_code, batch_number
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(&query.material_id)
        .bind(&query.bin_code)
        .bind(&query.batch_number)
        .bind(&query.zone)
        .bind(&query.quality_status)
        .bind(query.only_available)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        rows.into_iter()
            .map(Self::parse_current_stock_row)
            .collect()
    }

    async fn list_bin_stock(&self, query: CurrentStockQuery) -> AppResult<Vec<BinStock>> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT
                material_id,
                bin_code,
                batch_number,
                quality_status::TEXT as quality_status,
                qty,
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

        rows.into_iter().map(Self::parse_bin_stock_row).collect()
    }

    async fn list_transactions(
        &self,
        query: InventoryTransactionQuery,
    ) -> AppResult<Vec<InventoryTransaction>> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT
                transaction_id,
                movement_type::TEXT AS movement_type,
                material_id,
                quantity,
                from_bin,
                to_bin,
                batch_number,
                serial_number,
                reference_doc,
                operator,
                transaction_date,
                remarks
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

        rows.into_iter().map(Self::parse_transaction_row).collect()
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
                quantity,
                from_bin,
                to_bin,
                batch_number,
                serial_number,
                reference_doc,
                operator,
                transaction_date,
                remarks
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
    async fn list_batches(&self, query: BatchQuery) -> AppResult<Vec<Batch>> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT
                batch_number,
                material_id,
                production_date,
                expiry_date,
                quality_grade,
                current_stock,
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

        rows.into_iter().map(Self::parse_batch_row).collect()
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
                current_stock,
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
    ) -> AppResult<Vec<BatchHistory>> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT
                history_id,
                batch_number,
                material_id,
                event_type,
                old_quality_status::TEXT as old_quality_status,
                new_quality_status::TEXT as new_quality_status,
                old_bin,
                new_bin,
                old_stock,
                new_stock,
                transaction_id,
                changed_by,
                changed_at,
                remarks
            FROM wms.wms_batch_history
            WHERE batch_number = $1
              AND ($2::TEXT IS NULL OR event_type = $2)
              AND ($3::TEXT IS NULL OR changed_by = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR changed_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR changed_at <= $5)
            ORDER BY changed_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(batch_number)
        .bind(&query.event_type)
        .bind(&query.operator)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_error)?;

        rows.into_iter()
            .map(Self::parse_batch_history_row)
            .collect()
    }

    async fn pick_batch_fefo(&self, command: PickBatchFefoCommand) -> AppResult<Value> {
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
        .bind(command.quantity)
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
    async fn list_map_history(&self, query: MapHistoryQuery) -> AppResult<Vec<MapHistory>> {
        let page = query.page();

        let rows = sqlx::query(
            r#"
            SELECT
                history_id,
                material_id,
                old_map_price,
                new_map_price,
                old_stock_qty,
                new_stock_qty,
                incoming_qty,
                incoming_unit_price,
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

        rows.into_iter().map(Self::parse_map_history_row).collect()
    }

    async fn list_material_map_history(
        &self,
        material_id: String,
        mut query: MapHistoryQuery,
    ) -> AppResult<Vec<MapHistory>> {
        query.material_id = Some(material_id);
        self.list_map_history(query).await
    }
}
