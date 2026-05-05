use async_trait::async_trait;
use cuba_shared::AppResult;

use crate::domain::{
    BatchGenealogy, BomExplosionResult, ProductionCompleteResult, ProductionOrder,
    ProductionOrderId, ProductionOrderLine, ProductionVariance,
};

use super::{
    BomExplosionCommand, CompleteProductionOrderCommand, CreateProductionOrderCommand,
    ProductionOrderQuery, ProductionVarianceQuery, ReleaseProductionOrderCommand,
};

#[async_trait]
pub trait ProductionOrderRepository: Send + Sync {
    async fn create_order(&self, command: CreateProductionOrderCommand) -> AppResult<ProductionOrderId>;

    async fn find_by_id(&self, order_id: &str) -> AppResult<ProductionOrder>;

    async fn list(&self, query: ProductionOrderQuery) -> AppResult<Vec<ProductionOrder>>;

    async fn list_lines(&self, order_id: &str) -> AppResult<Vec<ProductionOrderLine>>;

    async fn release(&self, command: ReleaseProductionOrderCommand) -> AppResult<ProductionOrder>;

    async fn cancel(&self, order_id: &str, operator: Option<String>) -> AppResult<ProductionOrder>;

    async fn close(&self, order_id: &str, operator: Option<String>) -> AppResult<ProductionOrder>;
}

#[async_trait]
pub trait BomExplosionRepository: Send + Sync {
    async fn explode(&self, command: BomExplosionCommand) -> AppResult<BomExplosionResult>;
}

#[async_trait]
pub trait ProductionPostingRepository: Send + Sync {
    async fn complete_order(&self, command: CompleteProductionOrderCommand) -> AppResult<ProductionCompleteResult>;
}

#[async_trait]
pub trait BatchGenealogyRepository: Send + Sync {
    async fn find_by_order_id(&self, order_id: &str) -> AppResult<Vec<BatchGenealogy>>;

    async fn find_components_by_finished_batch(&self, batch_number: &str) -> AppResult<Vec<BatchGenealogy>>;

    async fn find_where_used_by_component_batch(&self, batch_number: &str) -> AppResult<Vec<BatchGenealogy>>;
}

#[async_trait]
pub trait ProductionVarianceRepository: Send + Sync {
    async fn find_by_order_id(&self, order_id: &str) -> AppResult<ProductionVariance>;

    async fn list(&self, query: ProductionVarianceQuery) -> AppResult<Vec<ProductionVariance>>;
}