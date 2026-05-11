use async_trait::async_trait;
use cuba_shared::AppResult;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderClosed,
    PurchaseOrderCreated, PurchaseOrderDetail, PurchaseOrderQuery, PurchaseOrderSummary,
    PurchaseReceiptPosted,
};

#[async_trait]
pub trait PurchaseOrderRepository: Send + Sync {
    async fn create_order(
        &self,
        command: CreatePurchaseOrderCommand,
        operator: String,
    ) -> AppResult<PurchaseOrderCreated>;

    async fn list_orders(&self, query: PurchaseOrderQuery) -> AppResult<Vec<PurchaseOrderSummary>>;

    async fn get_order(&self, po_id: String) -> AppResult<PurchaseOrderDetail>;

    async fn post_receipt(
        &self,
        command: PostPurchaseReceiptCommand,
        operator: String,
    ) -> AppResult<PurchaseReceiptPosted>;

    async fn close_order(&self, po_id: String, operator: String) -> AppResult<PurchaseOrderClosed>;
}
