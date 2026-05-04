use async_trait::async_trait;
use cuba_shared::AppResult;
use serde_json::Value;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderQuery,
};

#[async_trait]
pub trait PurchaseOrderRepository: Send + Sync {
    async fn create_order(
        &self,
        command: CreatePurchaseOrderCommand,
        operator: String,
    ) -> AppResult<Value>;

    async fn list_orders(&self, query: PurchaseOrderQuery) -> AppResult<Value>;

    async fn get_order(&self, po_id: String) -> AppResult<Value>;

    async fn post_receipt(
        &self,
        command: PostPurchaseReceiptCommand,
        operator: String,
    ) -> AppResult<Value>;

    async fn close_order(&self, po_id: String, operator: String) -> AppResult<Value>;
}
