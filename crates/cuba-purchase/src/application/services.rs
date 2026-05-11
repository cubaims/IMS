use std::sync::Arc;

use cuba_shared::{AppError, AppResult};
use rust_decimal::Decimal;
use validator::Validate;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderClosed,
    PurchaseOrderCreated, PurchaseOrderDetail, PurchaseOrderQuery, PurchaseOrderRepository,
    PurchaseOrderSummary, PurchaseReceiptPosted,
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
    ) -> AppResult<PurchaseOrderCreated> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;
        validate_create_order(&command)?;

        self.repository.create_order(command, operator).await
    }

    pub async fn list_orders(
        &self,
        query: PurchaseOrderQuery,
    ) -> AppResult<Vec<PurchaseOrderSummary>> {
        self.repository.list_orders(query).await
    }

    pub async fn get_order(&self, po_id: String) -> AppResult<PurchaseOrderDetail> {
        self.repository.get_order(po_id).await
    }

    pub async fn post_receipt(
        &self,
        command: PostPurchaseReceiptCommand,
        operator: String,
    ) -> AppResult<PurchaseReceiptPosted> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;
        validate_receipt(&command)?;

        self.repository.post_receipt(command, operator).await
    }

    pub async fn close_order(
        &self,
        po_id: String,
        operator: String,
    ) -> AppResult<PurchaseOrderClosed> {
        self.repository.close_order(po_id, operator).await
    }
}

fn validate_create_order(command: &CreatePurchaseOrderCommand) -> AppResult<()> {
    let mut line_numbers = std::collections::HashSet::new();

    for line in &command.lines {
        if !line_numbers.insert(line.line_no) {
            return Err(AppError::business(
                "PO_LINE_DUPLICATED",
                format!("采购订单行号重复: {}", line.line_no),
            ));
        }

        if line.unit_price < Decimal::ZERO {
            return Err(AppError::Validation(format!(
                "采购单价不能小于 0: line_no={}",
                line.line_no
            )));
        }
    }

    Ok(())
}

fn validate_receipt(command: &PostPurchaseReceiptCommand) -> AppResult<()> {
    let mut line_numbers = std::collections::HashSet::new();

    for line in &command.lines {
        if !line_numbers.insert(line.line_no) {
            return Err(AppError::business(
                "PO_LINE_DUPLICATED",
                format!("收货行号重复: {}", line.line_no),
            ));
        }
    }

    Ok(())
}
