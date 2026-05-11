use async_trait::async_trait;

use cuba_shared::AppResult;

use crate::{
    application::{
        BatchHistoryQuery, BatchQuery, CurrentStockQuery, InventoryTransactionQuery,
        MapHistoryQuery, PickBatchFefoCommand, PostInventoryCommand,
    },
    domain::{
        Batch, BatchHistory, BinStock, CurrentStock, InventoryPostingResult, InventoryTransaction,
        MapHistory,
    },
};

use super::Page;

#[async_trait]
pub trait InventoryRepository: Send + Sync {
    async fn post_inventory_transaction(
        &self,
        command: PostInventoryCommand,
        operator: String,
    ) -> AppResult<InventoryPostingResult>;

    async fn list_current_stock(&self, query: CurrentStockQuery) -> AppResult<Page<CurrentStock>>;

    async fn list_bin_stock(&self, query: CurrentStockQuery) -> AppResult<Page<BinStock>>;

    async fn stock_by_zone(&self, query: CurrentStockQuery) -> AppResult<serde_json::Value>;

    async fn bin_summary(&self, query: CurrentStockQuery) -> AppResult<serde_json::Value>;

    async fn batch_summary(&self, query: CurrentStockQuery) -> AppResult<serde_json::Value>;

    async fn list_transactions(
        &self,
        query: InventoryTransactionQuery,
    ) -> AppResult<Page<InventoryTransaction>>;

    async fn get_transaction(
        &self,
        transaction_id: String,
    ) -> AppResult<Option<InventoryTransaction>>;
}

#[async_trait]
pub trait BatchRepository: Send + Sync {
    async fn list_batches(&self, query: BatchQuery) -> AppResult<Page<Batch>>;

    async fn get_batch(&self, batch_number: String) -> AppResult<Option<Batch>>;

    async fn list_batch_history(
        &self,
        batch_number: String,
        query: BatchHistoryQuery,
    ) -> AppResult<Page<BatchHistory>>;

    async fn pick_batch_fefo(&self, command: PickBatchFefoCommand) -> AppResult<serde_json::Value>;
}

#[async_trait]
pub trait MapHistoryRepository: Send + Sync {
    async fn list_map_history(&self, query: MapHistoryQuery) -> AppResult<Page<MapHistory>>;

    async fn list_material_map_history(
        &self,
        material_id: String,
        query: MapHistoryQuery,
    ) -> AppResult<Page<MapHistory>>;
}
