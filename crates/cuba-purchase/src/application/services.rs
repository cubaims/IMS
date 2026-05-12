use std::sync::Arc;

use cuba_shared::{AppError, AppResult};
use rust_decimal::Decimal;
use validator::Validate;

use crate::application::{
    CreatePurchaseOrderCommand, PostPurchaseReceiptCommand, PurchaseOrderClosed,
    PurchaseOrderCreated, PurchaseOrderDetail, PurchaseOrderQuery, PurchaseOrderRepository,
    PurchaseOrderSummary, PurchaseOrderUpdated, PurchaseReceiptPosted, UpdatePurchaseOrderCommand,
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

    pub async fn update_order(
        &self,
        command: UpdatePurchaseOrderCommand,
        operator: String,
    ) -> AppResult<PurchaseOrderUpdated> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;
        validate_update_order(&command)?;

        self.repository.update_order(command, operator).await
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

        if line.line_no <= 0 {
            return Err(AppError::Validation(format!(
                "采购订单行号必须大于 0: line_no={}",
                line.line_no
            )));
        }

        if line.ordered_qty <= 0 {
            return Err(AppError::Validation(format!(
                "采购数量必须大于 0: line_no={}",
                line.line_no
            )));
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

fn validate_update_order(command: &UpdatePurchaseOrderCommand) -> AppResult<()> {
    if let Some(lines) = &command.lines {
        if lines.is_empty() {
            return Err(AppError::Validation("采购订单明细不能更新为空".to_string()));
        }

        let mut line_numbers = std::collections::HashSet::new();

        for line in lines {
            if !line_numbers.insert(line.line_no) {
                return Err(AppError::business(
                    "PO_LINE_DUPLICATED",
                    format!("采购订单行号重复: {}", line.line_no),
                ));
            }

            if line.line_no <= 0 {
                return Err(AppError::Validation(format!(
                    "采购订单行号必须大于 0: line_no={}",
                    line.line_no
                )));
            }

            if line.ordered_qty <= 0 {
                return Err(AppError::Validation(format!(
                    "采购数量必须大于 0: line_no={}",
                    line.line_no
                )));
            }

            if line.unit_price < Decimal::ZERO {
                return Err(AppError::Validation(format!(
                    "采购单价不能小于 0: line_no={}",
                    line.line_no
                )));
            }
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

        if line.line_no <= 0 {
            return Err(AppError::Validation(format!(
                "收货行号必须大于 0: line_no={}",
                line.line_no
            )));
        }

        if line.receipt_qty <= 0 {
            return Err(AppError::Validation(format!(
                "收货数量必须大于 0: line_no={}",
                line.line_no
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::CreatePurchaseOrderLineCommand;

    fn line(line_no: i32, ordered_qty: i32) -> CreatePurchaseOrderLineCommand {
        CreatePurchaseOrderLineCommand {
            line_no,
            material_id: "MAT-001".to_string(),
            ordered_qty,
            unit_price: Decimal::ONE,
            expected_bin: Some("BIN-A".to_string()),
        }
    }

    #[test]
    fn update_order_rejects_empty_line_replacement() {
        let command = UpdatePurchaseOrderCommand {
            po_id: "PO-1".to_string(),
            supplier_id: None,
            expected_date: None,
            remark: None,
            lines: Some(Vec::new()),
        };

        let err = validate_update_order(&command).expect_err("empty lines are invalid");

        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn update_order_rejects_duplicate_line_numbers() {
        let command = UpdatePurchaseOrderCommand {
            po_id: "PO-1".to_string(),
            supplier_id: None,
            expected_date: None,
            remark: None,
            lines: Some(vec![line(10, 1), line(10, 2)]),
        };

        let err = validate_update_order(&command).expect_err("duplicate lines are invalid");

        assert!(matches!(
            err,
            AppError::Business {
                code: "PO_LINE_DUPLICATED",
                ..
            }
        ));
    }

    #[test]
    fn create_order_rejects_non_positive_quantity() {
        let command = CreatePurchaseOrderCommand {
            supplier_id: "SUP-1".to_string(),
            expected_date: None,
            remark: None,
            lines: vec![line(10, 0)],
        };

        let err = validate_create_order(&command).expect_err("quantity must be positive");

        assert!(matches!(err, AppError::Validation(_)));
    }
}
