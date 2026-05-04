use std::sync::Arc;

use cuba_shared::AppResult;
use serde_json::Value;
use validator::Validate;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderQuery,
    PurchaseOrderRepository,
};

#[derive(Clone)]
pub struct PurchaseOrderService {
    repository: Arc<dyn PurchaseOrderRepository>,
}

impl PurchaseOrderService {
    pub fn new(repository: Arc<dyn PurchaseOrderRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_order(
        &self,
        command: CreatePurchaseOrderCommand,
        operator: String,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| cuba_shared::AppError::Validation(err.to_string()))?;

        self.repository.create_order(command, operator).await
    }

    pub async fn list_orders(&self, query: PurchaseOrderQuery) -> AppResult<Value> {
        self.repository.list_orders(query).await
    }

    pub async fn get_order(&self, po_id: String) -> AppResult<Value> {
        self.repository.get_order(po_id).await
    }

    pub async fn post_receipt(
        &self,
        command: PostPurchaseReceiptCommand,
        operator: String,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| cuba_shared::AppError::Validation(err.to_string()))?;

        self.repository.post_receipt(command, operator).await
    }

    pub async fn close_order(&self, po_id: String, operator: String) -> AppResult<Value> {
        self.repository.close_order(po_id, operator).await
    }
}
