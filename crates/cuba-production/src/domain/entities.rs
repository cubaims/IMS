use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use super::{
    BatchNumber, BinCode, BomId, MaterialId, ProductionOrderId, ProductionOrderStatus, VariantCode,
    WorkCenterId,
};
use crate::domain::ProductionDomainError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOrder {
    pub order_id: ProductionOrderId,
    pub variant_code: VariantCode,
    pub finished_material_id: MaterialId,
    pub bom_id: BomId,
    pub planned_qty: i32,
    pub completed_qty: i32,
    pub work_center_id: WorkCenterId,
    pub planned_start_date: Option<Date>,
    pub planned_end_date: Option<Date>,
    pub status: ProductionOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

impl ProductionOrder {
    pub fn release(&mut self, component_count: usize) -> Result<(), ProductionDomainError> {
        self.status.ensure_can_release()?;

        if component_count == 0 {
            return Err(ProductionDomainError::BomNoComponents);
        }

        self.status = ProductionOrderStatus::Released;
        Ok(())
    }

    pub fn start_or_complete(
        &mut self,
        completed_qty: i32,
    ) -> Result<ProductionCompletionDecision, ProductionDomainError> {
        self.status.ensure_can_complete()?;

        if completed_qty <= 0 {
            return Err(ProductionDomainError::ProductionQuantityInvalid);
        }

        let new_completed_qty = self
            .completed_qty
            .checked_add(completed_qty)
            .ok_or(ProductionDomainError::ProductionQuantityExceeded)?;

        if new_completed_qty > self.planned_qty {
            return Err(ProductionDomainError::ProductionQuantityExceeded);
        }

        self.completed_qty = new_completed_qty;
        self.status = if self.completed_qty >= self.planned_qty {
            ProductionOrderStatus::Completed
        } else {
            ProductionOrderStatus::InProduction
        };

        Ok(ProductionCompletionDecision {
            completed_qty,
            new_completed_qty: self.completed_qty,
            status: self.status,
            is_fully_completed: self.status == ProductionOrderStatus::Completed,
        })
    }

    pub fn cancel(&mut self) -> Result<(), ProductionDomainError> {
        self.status.ensure_can_cancel()?;
        self.status = ProductionOrderStatus::Cancelled;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), ProductionDomainError> {
        self.status.ensure_can_close()?;
        self.status = ProductionOrderStatus::Completed;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionCompletionDecision {
    pub completed_qty: i32,
    pub new_completed_qty: i32,
    pub status: ProductionOrderStatus,
    pub is_fully_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOrderLine {
    pub order_id: ProductionOrderId,
    pub line_no: i32,
    pub component_material_id: MaterialId,
    pub required_qty: i32,
    pub issued_qty: i32,
    pub source_bin: Option<BinCode>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomExplosionComponent {
    pub level: i32,
    pub parent_material_id: MaterialId,
    pub component_material_id: MaterialId,
    pub component_name: Option<String>,
    pub quantity_per: Decimal,
    pub required_qty: Decimal,
    pub available_qty: Decimal,
    pub net_requirement_qty: Decimal,
    pub is_shortage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomExplosionResult {
    pub variant_code: Option<VariantCode>,
    pub finished_material_id: MaterialId,
    pub quantity: i32,
    pub merge_components: bool,
    pub components: Vec<BomExplosionComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionCompleteTransaction {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: MaterialId,
    pub quantity: i32,
    pub batch_number: Option<BatchNumber>,
    pub from_bin: Option<BinCode>,
    pub to_bin: Option<BinCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionCompleteResult {
    pub order_id: ProductionOrderId,
    pub status: ProductionOrderStatus,
    pub completed_qty: i32,
    pub finished_transaction: Option<ProductionCompleteTransaction>,
    pub component_transactions: Vec<ProductionCompleteTransaction>,
    pub genealogy_count: i64,
    pub variance_id: Option<String>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenealogy {
    pub parent_batch_number: BatchNumber,
    pub component_batch_number: BatchNumber,
    pub parent_material_id: MaterialId,
    pub component_material_id: MaterialId,
    pub production_order_id: ProductionOrderId,
    pub consumed_qty: Decimal,
    pub output_qty: Decimal,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionVariance {
    pub order_id: ProductionOrderId,
    pub variant_code: Option<VariantCode>,
    pub output_material_id: MaterialId,
    pub planned_quantity: i32,
    pub actual_quantity: i32,
    pub planned_unit_cost: Decimal,
    pub actual_unit_cost: Decimal,
    pub planned_material_cost: Decimal,
    pub actual_material_cost: Decimal,
    pub material_variance: Decimal,
    pub labor_variance: Decimal,
    pub overhead_variance: Decimal,
    pub total_variance: Decimal,
    pub variance_pct: Option<Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn production_order(
        status: ProductionOrderStatus,
        planned_qty: i32,
        completed_qty: i32,
    ) -> ProductionOrder {
        ProductionOrder {
            order_id: ProductionOrderId("MO-001".to_string()),
            variant_code: VariantCode("VAR-A".to_string()),
            finished_material_id: MaterialId("MAT-FG".to_string()),
            bom_id: BomId("BOM-001".to_string()),
            planned_qty,
            completed_qty,
            work_center_id: WorkCenterId("WC-001".to_string()),
            planned_start_date: None,
            planned_end_date: None,
            status,
            remark: None,
            created_by: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn release_requires_planned_order_with_components() {
        let mut order = production_order(ProductionOrderStatus::Planned, 100, 0);

        order.release(1).expect("planned order can release");

        assert_eq!(order.status, ProductionOrderStatus::Released);
    }

    #[test]
    fn release_without_components_is_rejected() {
        let mut order = production_order(ProductionOrderStatus::Planned, 100, 0);

        let err = order.release(0).expect_err("components are required");

        assert!(matches!(err, ProductionDomainError::BomNoComponents));
    }

    #[test]
    fn partial_completion_moves_order_to_in_production() {
        let mut order = production_order(ProductionOrderStatus::Released, 100, 0);

        let decision = order
            .start_or_complete(30)
            .expect("released order can be partially completed");

        assert_eq!(order.completed_qty, 30);
        assert_eq!(decision.status, ProductionOrderStatus::InProduction);
        assert!(!decision.is_fully_completed);
    }

    #[test]
    fn final_completion_completes_order() {
        let mut order = production_order(ProductionOrderStatus::InProduction, 100, 70);

        let decision = order
            .start_or_complete(30)
            .expect("remaining quantity can be completed");

        assert_eq!(order.completed_qty, 100);
        assert_eq!(decision.status, ProductionOrderStatus::Completed);
        assert!(decision.is_fully_completed);
    }

    #[test]
    fn completion_cannot_exceed_planned_quantity() {
        let mut order = production_order(ProductionOrderStatus::Released, 100, 70);

        let err = order
            .start_or_complete(31)
            .expect_err("completion cannot exceed remaining quantity");

        assert!(matches!(
            err,
            ProductionDomainError::ProductionQuantityExceeded
        ));
    }
}
