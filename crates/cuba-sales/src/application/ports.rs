use async_trait::async_trait;
use cuba_shared::AppResult;
use serde_json::Value;

use crate::application::{
    CreateSalesOrderCommand, PostSalesShipmentCommand, PreviewSalesFefoPickCommand,
    SalesOrderQuery, UpdateSalesOrderCommand,
};

#[async_trait]
pub trait SalesOrderRepository: Send + Sync {
    async fn create_order(
        &self,
        command: CreateSalesOrderCommand,
        operator: String,
    ) -> AppResult<Value>;

    async fn update_order(
        &self,
        command: UpdateSalesOrderCommand,
        operator: String,
    ) -> AppResult<Value>;

    async fn list_orders(&self, query: SalesOrderQuery) -> AppResult<Value>;

    async fn get_order(&self, so_id: String) -> AppResult<Value>;

    async fn post_shipment(
        &self,
        command: PostSalesShipmentCommand,
        operator: String,
    ) -> AppResult<Value>;

    async fn preview_fefo_pick(&self, command: PreviewSalesFefoPickCommand) -> AppResult<Value>;

    async fn close_order(&self, so_id: String, operator: String) -> AppResult<Value>;
}
