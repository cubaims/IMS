use std::sync::Arc;

use cuba_shared::AppResult;
use serde_json::Value;
use validator::Validate;

use crate::application::{
    CreateSalesOrderCommand, PostSalesShipmentCommand, PreviewSalesFefoPickCommand,
    SalesOrderQuery, SalesOrderRepository,
};

#[derive(Clone)]
pub struct SalesOrderService {
    repository: Arc<dyn SalesOrderRepository>,
}

impl SalesOrderService {
    pub fn new(repository: Arc<dyn SalesOrderRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_order(
        &self,
        command: CreateSalesOrderCommand,
        operator: String,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| cuba_shared::AppError::Validation(err.to_string()))?;

        self.repository.create_order(command, operator).await
    }

    pub async fn list_orders(&self, query: SalesOrderQuery) -> AppResult<Value> {
        self.repository.list_orders(query).await
    }

    pub async fn get_order(&self, so_id: String) -> AppResult<Value> {
        self.repository.get_order(so_id).await
    }

    pub async fn post_shipment(
        &self,
        command: PostSalesShipmentCommand,
        operator: String,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| cuba_shared::AppError::Validation(err.to_string()))?;

        self.repository.post_shipment(command, operator).await
    }

    pub async fn preview_fefo_pick(
        &self,
        command: PreviewSalesFefoPickCommand,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| cuba_shared::AppError::Validation(err.to_string()))?;

        self.repository.preview_fefo_pick(command).await
    }

    pub async fn close_order(&self, so_id: String, operator: String) -> AppResult<Value> {
        self.repository.close_order(so_id, operator).await
    }
}
