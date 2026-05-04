use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{
    BatchNumber, BomId, MaterialId, ProductionDomainError, ProductionOrderId,
    ProductionOrderStatus, ProductionQuantity, VariantCode, WorkCenterId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOrder {
    pub order_id: ProductionOrderId,
    pub variant_code: VariantCode,
    pub finished_material_id: MaterialId,
    pub bom_id: BomId,
    pub planned_qty: ProductionQuantity,
    pub completed_qty: i32,
    pub work_center_id: WorkCenterId,
    pub planned_start_date: Option<NaiveDate>,
    pub planned_end_date: Option<NaiveDate>,
    pub status: ProductionOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub lines: Vec<ProductionOrderLine>,
}

impl ProductionOrder {
    pub fn ensure_can_release(&self) -> Result<(), ProductionDomainError> {
        if !self.status.can_release() {
            return Err(ProductionDomainError::ProductionOrderStatusInvalid(
                self.status.as_db_text().to_string(),
            ));
        }

        if self.lines.is_empty() {
            return Err(ProductionDomainError::BomNoComponents(
                self.bom_id.as_str().to_string(),
            ));
        }

        Ok(())
    }

    pub fn ensure_can_complete(&self, completed_qty: i32) -> Result<(), ProductionDomainError> {
        if !self.status.can_complete() {
            return Err(ProductionDomainError::ProductionOrderStatusInvalid(
                self.status.as_db_text().to_string(),
            ));
        }

        if completed_qty <= 0 {
            return Err(ProductionDomainError::InvalidCompletedQuantity);
        }

        let remaining = self.planned_qty.value() - self.completed_qty;

        if completed_qty > remaining {
            return Err(ProductionDomainError::CompletedQuantityExceeded);
        }

        Ok(())
    }

    pub fn next_status_after_completion(&self, completed_qty: i32) -> ProductionOrderStatus {
        let total_completed = self.completed_qty + completed_qty;

        if total_completed >= self.planned_qty.value() {
            ProductionOrderStatus::Completed
        } else {
            ProductionOrderStatus::PartiallyCompleted
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOrderLine {
    pub line_no: i32,
    pub component_material_id: MaterialId,
    pub required_qty: i32,
    pub issued_qty: i32,
    pub source_bin: Option<String>,
    pub batch_number: Option<BatchNumber>,
}

impl ProductionOrderLine {
    pub fn remaining_qty(&self) -> i32 {
        self.required_qty - self.issued_qty
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomExplosion {
    pub variant_code: Option<VariantCode>,
    pub finished_material_id: MaterialId,
    pub quantity: ProductionQuantity,
    pub components: Vec<BomComponentRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomComponentRequirement {
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
pub struct ProductionCompleteResult {
    pub order_id: ProductionOrderId,
    pub status: ProductionOrderStatus,
    pub completed_qty: i32,
    pub finished_transaction: Option<ProductionTransaction>,
    pub component_transactions: Vec<ProductionTransaction>,
    pub genealogy_count: i64,
    pub variance_id: Option<i64>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionTransaction {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: MaterialId,
    pub quantity: i32,
    pub batch_number: Option<BatchNumber>,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenealogy {
    pub parent_batch_number: BatchNumber,
    pub component_batch_number: BatchNumber,
    pub parent_material_id: MaterialId,
    pub component_material_id: MaterialId,
    pub production_order_id: ProductionOrderId,
    pub consumed_qty: i32,
    pub output_qty: i32,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionVariance {
    pub variance_id: i64,
    pub order_id: ProductionOrderId,
    pub variant_code: Option<VariantCode>,
    pub output_material_id: MaterialId,
    pub planned_quantity: i32,
    pub actual_quantity: i32,
    pub planned_unit_cost: Decimal,
    pub actual_unit_cost: Decimal,
    pub planned_material_cost: Decimal,
    pub actual_material_cost: Decimal,
    pub labor_variance: Decimal,
    pub overhead_variance: Decimal,
    pub material_variance: Decimal,
    pub total_variance: Decimal,
    pub variance_pct: Option<Decimal>,
    pub variance_reason: Option<String>,
    pub calculated_at: Option<DateTime<Utc>>,
}