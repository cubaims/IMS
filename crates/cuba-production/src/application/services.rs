use std::sync::Arc;

use serde_json::Value;
use validator::Validate;

use cuba_shared::{AppError, AppResult};

use crate::application::{
    BatchGenealogyRepository, BomExplosionRepository,
    CompleteProductionOrderCommand, CreateProductionOrderCommand,
    CreateProductionOrderResult, ListProductionOrdersQuery,
    ListProductionVariancesQuery, PreviewBomExplosionCommand,
    ProductionCompleteAppResult, ProductionOrderRepository,
    ProductionPostingRepository, ProductionVarianceRepository,
    ReleaseProductionOrderCommand, ReleaseProductionOrderResult,
};

#[derive(Clone)]
pub struct ProductionService {
    production_orders: Arc<dyn ProductionOrderRepository>,
    bom_explosion: Arc<dyn BomExplosionRepository>,
    production_posting: Arc<dyn ProductionPostingRepository>,
    batch_genealogy: Arc<dyn BatchGenealogyRepository>,
    production_variance: Arc<dyn ProductionVarianceRepository>,
}

impl ProductionService {
    pub fn new(
        production_orders: Arc<dyn ProductionOrderRepository>,
        bom_explosion: Arc<dyn BomExplosionRepository>,
        production_posting: Arc<dyn ProductionPostingRepository>,
        batch_genealogy: Arc<dyn BatchGenealogyRepository>,
        production_variance: Arc<dyn ProductionVarianceRepository>,
    ) -> Self {
        Self {
            production_orders,
            bom_explosion,
            production_posting,
            batch_genealogy,
            production_variance,
        }
    }

    pub async fn preview_bom_explosion(
        &self,
        command: PreviewBomExplosionCommand,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        self.bom_explosion
            .preview_bom_explosion(command)
            .await
    }

    pub async fn create_order(
        &self,
        command: CreateProductionOrderCommand,
    ) -> AppResult<CreateProductionOrderResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        if let (Some(start), Some(end)) =
            (command.planned_start_date, command.planned_end_date)
        {
            if end < start {
                return Err(AppError::Validation(
                    "planned_end_date must be greater than or equal to planned_start_date"
                        .to_string(),
                ));
            }
        }

        self.production_orders
            .create_order(command)
            .await
    }

    pub async fn list_orders(
        &self,
        query: ListProductionOrdersQuery,
    ) -> AppResult<Value> {
        self.production_orders
            .list_orders(query)
            .await
    }

    pub async fn get_order(
        &self,
        order_id: String,
    ) -> AppResult<Value> {
        if order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.production_orders
            .get_order(order_id)
            .await
    }

    pub async fn release_order(
        &self,
        command: ReleaseProductionOrderCommand,
    ) -> AppResult<ReleaseProductionOrderResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        if command.order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.production_orders
            .release_order(command)
            .await
    }

    pub async fn cancel_order(
        &self,
        order_id: String,
        remark: Option<String>,
    ) -> AppResult<Value> {
        if order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.production_orders
            .cancel_order(order_id, remark)
            .await
    }

    pub async fn close_order(
        &self,
        order_id: String,
        remark: Option<String>,
    ) -> AppResult<Value> {
        if order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.production_orders
            .close_order(order_id, remark)
            .await
    }

    pub async fn complete_order(
        &self,
        command: CompleteProductionOrderCommand,
    ) -> AppResult<ProductionCompleteAppResult> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;

        if command.pick_strategy != "FEFO" {
            return Err(AppError::Validation(
                "only FEFO pick strategy is supported in Phase 6 MVP".to_string(),
            ));
        }

        self.production_posting
            .complete_order(command)
            .await
    }

    pub async fn get_order_components(
        &self,
        order_id: String,
    ) -> AppResult<Value> {
        if order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.bom_explosion
            .get_order_components(order_id)
            .await
    }

    pub async fn get_order_genealogy(
        &self,
        order_id: String,
    ) -> AppResult<Value> {
        if order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.batch_genealogy
            .get_order_genealogy(order_id)
            .await
    }

    pub async fn get_components_by_finished_batch(
        &self,
        batch_number: String,
    ) -> AppResult<Value> {
        if batch_number.trim().is_empty() {
            return Err(AppError::Validation(
                "batch_number is required".to_string(),
            ));
        }

        self.batch_genealogy
            .get_components_by_finished_batch(batch_number)
            .await
    }

    pub async fn get_where_used_by_component_batch(
        &self,
        batch_number: String,
    ) -> AppResult<Value> {
        if batch_number.trim().is_empty() {
            return Err(AppError::Validation(
                "batch_number is required".to_string(),
            ));
        }

        self.batch_genealogy
            .get_where_used_by_component_batch(batch_number)
            .await
    }

    pub async fn get_order_variance(
        &self,
        order_id: String,
    ) -> AppResult<Value> {
        if order_id.trim().is_empty() {
            return Err(AppError::Validation(
                "order_id is required".to_string(),
            ));
        }

        self.production_variance
            .get_order_variance(order_id)
            .await
    }

    pub async fn list_variances(
        &self,
        query: ListProductionVariancesQuery,
    ) -> AppResult<Value> {
        self.production_variance
            .list_variances(query)
            .await
    }
}