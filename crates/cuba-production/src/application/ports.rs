use async_trait::async_trait;
use serde_json::Value;

use cuba_shared::AppResult;

use crate::application::{
    CompleteProductionOrderCommand, CreateProductionOrderCommand,
    CreateProductionOrderResult, ListProductionOrdersQuery,
    ListProductionVariancesQuery, PreviewBomExplosionCommand,
    ProductionCompleteAppResult, ReleaseProductionOrderCommand,
    ReleaseProductionOrderResult,
};

#[async_trait]
pub trait ProductionOrderRepository: Send + Sync {
    async fn create_order(
        &self,
        command: CreateProductionOrderCommand,
    ) -> AppResult<CreateProductionOrderResult>;

    async fn list_orders(
        &self,
        query: ListProductionOrdersQuery,
    ) -> AppResult<Value>;

    async fn get_order(
        &self,
        order_id: String,
    ) -> AppResult<Value>;

    async fn release_order(
        &self,
        command: ReleaseProductionOrderCommand,
    ) -> AppResult<ReleaseProductionOrderResult>;

    async fn cancel_order(
        &self,
        order_id: String,
        remark: Option<String>,
    ) -> AppResult<Value>;

    async fn close_order(
        &self,
        order_id: String,
        remark: Option<String>,
    ) -> AppResult<Value>;
}

#[async_trait]
pub trait BomExplosionRepository: Send + Sync {
    async fn preview_bom_explosion(
        &self,
        command: PreviewBomExplosionCommand,
    ) -> AppResult<Value>;

    async fn get_order_components(
        &self,
        order_id: String,
    ) -> AppResult<Value>;
}

#[async_trait]
pub trait ProductionPostingRepository: Send + Sync {
    async fn complete_order(
        &self,
        command: CompleteProductionOrderCommand,
    ) -> AppResult<ProductionCompleteAppResult>;
}

#[async_trait]
pub trait BatchGenealogyRepository: Send + Sync {
    async fn get_order_genealogy(
        &self,
        order_id: String,
    ) -> AppResult<Value>;

    async fn get_components_by_finished_batch(
        &self,
        batch_number: String,
    ) -> AppResult<Value>;

    async fn get_where_used_by_component_batch(
        &self,
        batch_number: String,
    ) -> AppResult<Value>;
}

#[async_trait]
pub trait ProductionVarianceRepository: Send + Sync {
    async fn get_order_variance(
        &self,
        order_id: String,
    ) -> AppResult<Value>;

    async fn list_variances(
        &self,
        query: ListProductionVariancesQuery,
    ) -> AppResult<Value>;
}