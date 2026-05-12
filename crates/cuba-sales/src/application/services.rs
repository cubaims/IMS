use std::sync::Arc;

use cuba_shared::{AppError, AppResult};
use rust_decimal::Decimal;
use serde_json::Value;
use validator::Validate;

use crate::application::{
    CreateSalesOrderCommand, PostSalesShipmentCommand, PreviewSalesFefoPickCommand,
    SalesOrderQuery, SalesOrderRepository, UpdateSalesOrderCommand,
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

    pub async fn update_order(
        &self,
        command: UpdateSalesOrderCommand,
        operator: String,
    ) -> AppResult<Value> {
        command
            .validate()
            .map_err(|err| AppError::Validation(err.to_string()))?;
        validate_update_order(&command)?;

        self.repository.update_order(command, operator).await
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

        if line.line_no <= 0 {
            return Err(AppError::Validation(format!(
                "销售订单行号必须大于 0: line_no={}",
                line.line_no
            )));
        }

        if line.ordered_qty <= 0 {
            return Err(AppError::Validation(format!(
                "销售数量必须大于 0: line_no={}",
                line.line_no
            )));
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

fn validate_update_order(command: &UpdateSalesOrderCommand) -> AppResult<()> {
    if let Some(lines) = &command.lines {
        if lines.is_empty() {
            return Err(AppError::Validation("销售订单明细不能更新为空".to_string()));
        }

        let mut line_numbers = std::collections::HashSet::new();

        for line in lines {
            if !line_numbers.insert(line.line_no) {
                return Err(AppError::business(
                    "SO_LINE_DUPLICATED",
                    format!("销售订单行号重复: {}", line.line_no),
                ));
            }

            if line.line_no <= 0 {
                return Err(AppError::Validation(format!(
                    "销售订单行号必须大于 0: line_no={}",
                    line.line_no
                )));
            }

            if line.ordered_qty <= 0 {
                return Err(AppError::Validation(format!(
                    "销售数量必须大于 0: line_no={}",
                    line.line_no
                )));
            }

            if line.unit_price < Decimal::ZERO {
                return Err(AppError::Validation(format!(
                    "销售单价不能小于 0: line_no={}",
                    line.line_no
                )));
            }
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

        if line.line_no <= 0 {
            return Err(AppError::Validation(format!(
                "发货行号必须大于 0: line_no={}",
                line.line_no
            )));
        }

        if line.shipment_qty <= 0 {
            return Err(AppError::Validation(format!(
                "发货数量必须大于 0: line_no={}",
                line.line_no
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{CreateSalesOrderLineCommand, PostSalesShipmentLineCommand};

    fn line(line_no: i32, ordered_qty: i32) -> CreateSalesOrderLineCommand {
        CreateSalesOrderLineCommand {
            line_no,
            material_id: "MAT-001".to_string(),
            ordered_qty,
            unit_price: Decimal::ONE,
            from_bin: Some("BIN-A".to_string()),
        }
    }

    #[test]
    fn update_order_rejects_empty_line_replacement() {
        let command = UpdateSalesOrderCommand {
            so_id: "SO-1".to_string(),
            customer_id: None,
            required_date: None,
            remark: None,
            lines: Some(Vec::new()),
        };

        let err = validate_update_order(&command).expect_err("empty lines are invalid");

        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn update_order_rejects_duplicate_line_numbers() {
        let command = UpdateSalesOrderCommand {
            so_id: "SO-1".to_string(),
            customer_id: None,
            required_date: None,
            remark: None,
            lines: Some(vec![line(10, 1), line(10, 2)]),
        };

        let err = validate_update_order(&command).expect_err("duplicate lines are invalid");

        assert!(matches!(
            err,
            AppError::Business {
                code: "SO_LINE_DUPLICATED",
                ..
            }
        ));
    }

    #[test]
    fn create_order_rejects_non_positive_quantity() {
        let command = CreateSalesOrderCommand {
            customer_id: "CUST-1".to_string(),
            required_date: None,
            remark: None,
            lines: vec![line(10, 0)],
        };

        let err = validate_create_order(&command).expect_err("quantity must be positive");

        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn shipment_rejects_unknown_pick_strategy() {
        let command = PostSalesShipmentCommand {
            so_id: "SO-1".to_string(),
            posting_date: None,
            pick_strategy: Some("RANDOM".to_string()),
            remark: None,
            lines: vec![PostSalesShipmentLineCommand {
                line_no: 10,
                shipment_qty: 1,
                batch_number: None,
                from_bin: None,
            }],
        };

        let err = validate_shipment(&command).expect_err("unknown strategy is invalid");

        assert!(matches!(err, AppError::Validation(_)));
    }
}
