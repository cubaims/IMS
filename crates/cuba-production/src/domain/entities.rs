use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{BatchNumber, BinCode, BomId, MaterialId, ProductionOrderId, ProductionOrderStatus, VariantCode, WorkCenterId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOrder {
    pub order_id: ProductionOrderId,
    pub variant_code: VariantCode,
    pub finished_material_id: MaterialId,
    pub bom_id: BomId,
    pub planned_qty: i32,
    pub completed_qty: i32,
    pub work_center_id: WorkCenterId,
    pub planned_start_date: Option<NaiveDate>,
    pub planned_end_date: Option<NaiveDate>,
    pub status: ProductionOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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