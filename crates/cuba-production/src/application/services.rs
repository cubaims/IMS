use std::sync::Arc;

use cuba_shared::{AppError, AppResult};
use validator::Validate;

use crate::domain::{
    BatchGenealogy, BomExplosionResult, ProductionCompleteResult, ProductionOrder,
    ProductionOrderId, ProductionOrderLine, ProductionVariance,
};

use super::{
    BatchGenealogyRepository, BomExplosionCommand, BomExplosionRepository,
    CompleteProductionOrderCommand, CreateProductionOrderCommand, ProductionOrderQuery,
    ProductionOrderRepository, ProductionPostingRepository, ProductionVarianceQuery,
    ProductionVarianceRepository, ReleaseProductionOrderCommand,
};

#[derive(Clone)]
pub struct ProductionService {
    production_orders: Arc<dyn ProductionOrderRepository>,
    bom_explosion: Arc<dyn BomExplosionRepository>,
    production_posting: Arc<dyn ProductionPostingRepository>,
    genealogy: Arc<dyn BatchGenealogyRepository>,
    variances: Arc<dyn ProductionVarianceRepository>,
}

impl ProductionService {
    pub fn new(
        production_orders: Arc<dyn ProductionOrderRepository>,
        bom_explosion: Arc<dyn BomExplosionRepository>,
        production_posting: Arc<dyn ProductionPostingRepository>,
        genealogy: Arc<dyn BatchGenealogyRepository>,
        variances: Arc<dyn ProductionVarianceRepository>,
    ) -> Self {
        Self {
            production_orders,
            bom_explosion,
            production_posting,
            genealogy,
            variances,
        }
    }

    pub async fn create_order(
        &self,
        command: CreateProductionOrderCommand,
    ) -> AppResult<ProductionOrderId> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.production_orders.create_order(command).await
    }

    pub async fn release_order(
        &self,
        command: ReleaseProductionOrderCommand,
    ) -> AppResult<ProductionOrder> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.production_orders.release(command).await
    }

    pub async fn complete_order(
        &self,
        command: CompleteProductionOrderCommand,
    ) -> AppResult<ProductionCompleteResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.production_posting.complete_order(command).await
    }

    pub async fn explode_bom(&self, command: BomExplosionCommand) -> AppResult<BomExplosionResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.bom_explosion.explode(command).await
    }

    pub async fn get_order(&self, order_id: &str) -> AppResult<ProductionOrder> {
        self.production_orders.find_by_id(order_id).await
    }

    pub async fn list_orders(
        &self,
        query: ProductionOrderQuery,
    ) -> AppResult<Vec<ProductionOrder>> {
        self.production_orders.list(query).await
    }

    pub async fn list_order_lines(&self, order_id: &str) -> AppResult<Vec<ProductionOrderLine>> {
        self.production_orders.list_lines(order_id).await
    }

    pub async fn get_genealogy(&self, order_id: &str) -> AppResult<Vec<BatchGenealogy>> {
        self.genealogy.find_by_order_id(order_id).await
    }

    pub async fn get_variance(&self, order_id: &str) -> AppResult<ProductionVariance> {
        self.variances.find_by_order_id(order_id).await
    }

    pub async fn list_variances(
        &self,
        query: ProductionVarianceQuery,
    ) -> AppResult<Vec<ProductionVariance>> {
        self.variances.list(query).await
    }
    pub async fn cancel_order(
        &self,
        order_id: &str,
        operator: Option<String>,
    ) -> AppResult<ProductionOrder> {
        self.production_orders.cancel(order_id, operator).await
    }

    pub async fn close_order(
        &self,
        order_id: &str,
        operator: Option<String>,
    ) -> AppResult<ProductionOrder> {
        self.production_orders.close(order_id, operator).await
    }
}
