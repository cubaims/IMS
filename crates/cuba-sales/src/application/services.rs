use std::sync::Arc;

use cuba_shared::{AppError, AppResult};
use rust_decimal::Decimal;
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
            .map_err(|err| AppError::Validation(err.to_string()))?;
        validate_create_order(&command)?;

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
            .map_err(|err| AppError::Validation(err.to_string()))?;
        validate_shipment(&command)?;

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

fn validate_create_order(command: &CreateSalesOrderCommand) -> AppResult<()> {
    let mut line_numbers = std::collections::HashSet::new();

    for line in &command.lines {
        if !line_numbers.insert(line.line_no) {
            return Err(AppError::business(
                "SO_LINE_DUPLICATED",
                format!("销售订单行号重复: {}", line.line_no),
            ));
        }

        if line.unit_price < Decimal::ZERO {
            return Err(AppError::Validation(format!(
                "销售单价不能小于 0: line_no={}",
                line.line_no
            )));
        }
    }

    Ok(())
}

fn validate_shipment(command: &PostSalesShipmentCommand) -> AppResult<()> {
    let mut line_numbers = std::collections::HashSet::new();

    let strategy = command
        .pick_strategy
        .as_deref()
        .unwrap_or("FEFO")
        .to_uppercase();

    if !matches!(strategy.as_str(), "FEFO" | "MANUAL") {
        return Err(AppError::Validation(format!(
            "不支持的发货策略: {}",
            strategy
        )));
    }

    for line in &command.lines {
        if !line_numbers.insert(line.line_no) {
            return Err(AppError::business(
                "SO_LINE_DUPLICATED",
                format!("发货行号重复: {}", line.line_no),
            ));
        }
    }

    Ok(())
}
