use std::sync::Arc;

use cuba_shared::{AppError, AppResult};
use validator::Validate;

use crate::{
    application::{
        BatchHistoryQuery, BatchQuery, CurrentStockQuery, InventoryRepository,
        InventoryTransactionQuery, MapHistoryQuery, MapHistoryRepository, PickBatchFefoCommand,
        PostInventoryCommand, TransferInventoryCommand,
    },
    domain::{
        Batch, BatchHistory, BinStock, CurrentStock, InventoryPostingResult,
        InventoryTransaction, MapHistory,
    },
};

use super::BatchRepository;

#[derive(Clone)]
pub struct InventoryService {
    inventory_repo: Arc<dyn InventoryRepository>,
    batch_repo: Arc<dyn BatchRepository>,
    map_history_repo: Arc<dyn MapHistoryRepository>,
}

impl InventoryService {
    pub fn new(
        inventory_repo: Arc<dyn InventoryRepository>,
        batch_repo: Arc<dyn BatchRepository>,
        map_history_repo: Arc<dyn MapHistoryRepository>,
    ) -> Self {
        Self {
            inventory_repo,
            batch_repo,
            map_history_repo,
        }
    }

    pub async fn post_inventory(
        &self,
        command: PostInventoryCommand,
        operator: String,
    ) -> AppResult<InventoryPostingResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        command
            .to_domain()
            .map_err(AppError::Validation)?
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.inventory_repo
            .post_inventory_transaction(command, operator)
            .await
    }

    pub async fn transfer_inventory(
        &self,
        command: TransferInventoryCommand,
        operator: String,
    ) -> AppResult<InventoryPostingResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        if command.from_bin == command.to_bin {
            return Err(AppError::Validation(
                "from_bin and to_bin cannot be the same".to_string(),
            ));
        }

        let post_command = command.into_post_command();

        post_command
            .to_domain()
            .map_err(AppError::Validation)?
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.inventory_repo
            .post_inventory_transaction(post_command, operator)
            .await
    }

    pub async fn list_current_stock(
        &self,
        query: CurrentStockQuery,
    ) -> AppResult<Vec<CurrentStock>> {
        self.inventory_repo.list_current_stock(query).await
    }

    pub async fn list_bin_stock(
        &self,
        query: CurrentStockQuery,
    ) -> AppResult<Vec<BinStock>> {
        self.inventory_repo.list_bin_stock(query).await
    }

    pub async fn list_transactions(
        &self,
        query: InventoryTransactionQuery,
    ) -> AppResult<Vec<InventoryTransaction>> {
        self.inventory_repo.list_transactions(query).await
    }

    pub async fn get_transaction(
        &self,
        transaction_id: String,
    ) -> AppResult<InventoryTransaction> {
        self.inventory_repo
            .get_transaction(transaction_id.clone())
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "inventory transaction {} not found",
                    transaction_id
                ))
            })
    }

    pub async fn list_batches(&self, query: BatchQuery) -> AppResult<Vec<Batch>> {
        self.batch_repo.list_batches(query).await
    }

    pub async fn get_batch(&self, batch_number: String) -> AppResult<Batch> {
        self.batch_repo
            .get_batch(batch_number.clone())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("batch {} not found", batch_number)))
    }

    pub async fn list_batch_history(
        &self,
        batch_number: String,
        query: BatchHistoryQuery,
    ) -> AppResult<Vec<BatchHistory>> {
        self.batch_repo
            .list_batch_history(batch_number, query)
            .await
    }

    pub async fn list_map_history(
        &self,
        query: MapHistoryQuery,
    ) -> AppResult<Vec<MapHistory>> {
        self.map_history_repo.list_map_history(query).await
    }

    pub async fn list_material_map_history(
        &self,
        material_id: String,
        query: MapHistoryQuery,
    ) -> AppResult<Vec<MapHistory>> {
        self.map_history_repo
            .list_material_map_history(material_id, query)
            .await
    }

    pub async fn pick_batch_fefo(
        &self,
        command: PickBatchFefoCommand,
    ) -> AppResult<serde_json::Value> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.batch_repo.pick_batch_fefo(command).await
    }
}