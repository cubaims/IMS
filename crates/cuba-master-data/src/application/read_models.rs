use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationAck {
    pub resource_id: String,
    pub affected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAck {
    pub resource_id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialReadModel {
    pub material_id: String,
    pub material_name: String,
    pub material_type: String,
    pub base_unit: String,
    pub default_zone: String,
    pub safety_stock: i32,
    pub reorder_point: i32,
    pub standard_price: Decimal,
    pub map_price: Decimal,
    pub current_stock: i32,
    pub quality_status: String,
    pub status: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageBinReadModel {
    pub bin_code: String,
    pub zone: String,
    pub bin_type: String,
    pub capacity: i32,
    pub current_occupied: i32,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinCapacityUtilizationReadModel {
    pub bin_code: String,
    pub zone: String,
    pub capacity: i32,
    pub current_occupied: i32,
    pub utilization_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierReadModel {
    pub supplier_id: String,
    pub supplier_name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub quality_rating: Option<String>,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerReadModel {
    pub customer_id: String,
    pub customer_name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub credit_limit: Option<Decimal>,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSupplierReadModel {
    pub id: i64,
    pub material_id: String,
    pub supplier_id: String,
    pub supplier_name: Option<String>,
    pub is_primary: Option<bool>,
    pub supplier_material_code: Option<String>,
    pub purchase_price: Option<Decimal>,
    pub currency: Option<String>,
    pub lead_time_days: Option<i32>,
    pub moq: Option<i32>,
    pub quality_rating: Option<String>,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariantReadModel {
    pub variant_code: String,
    pub variant_name: String,
    pub base_material_id: String,
    pub bom_id: Option<String>,
    pub standard_cost: Decimal,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomSummaryReadModel {
    pub bom_id: String,
    pub bom_name: String,
    pub parent_material_id: String,
    pub parent_material_name: Option<String>,
    pub variant_code: Option<String>,
    pub version: String,
    pub base_quantity: Decimal,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub created_by: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<OffsetDateTime>,
    pub notes: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
    pub component_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomHeaderReadModel {
    pub bom_id: String,
    pub bom_name: String,
    pub parent_material_id: String,
    pub variant_code: Option<String>,
    pub version: String,
    pub base_quantity: Decimal,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub created_by: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<OffsetDateTime>,
    pub notes: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomDetailReadModel {
    pub header: BomHeaderReadModel,
    pub components: Vec<BomComponentReadModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomComponentReadModel {
    pub id: i64,
    pub bom_id: String,
    pub parent_material_id: String,
    pub parent_material_name: Option<String>,
    pub component_material_id: String,
    pub component_material_name: Option<String>,
    pub quantity: Decimal,
    pub unit: String,
    pub bom_level: i32,
    pub scrap_rate: Option<Decimal>,
    pub is_critical: Option<bool>,
    pub valid_from: Option<Date>,
    pub valid_to: Option<Date>,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomTreeReadModel {
    pub bom_id: String,
    pub bom_name: String,
    pub parent_material_id: String,
    pub variant_code: Option<String>,
    pub version: String,
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub components: Vec<BomTreeComponentReadModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomTreeComponentReadModel {
    pub id: i64,
    pub component_material_id: String,
    pub component_material_name: Option<String>,
    pub quantity: Decimal,
    pub unit: String,
    pub bom_level: i32,
    pub scrap_rate: Option<Decimal>,
    pub is_critical: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomValidationReadModel {
    pub bom_id: String,
    pub header_exists: bool,
    pub component_count: i64,
    pub has_components: bool,
    pub self_reference_count: i64,
    pub missing_component_materials: i64,
    pub cycle_detected: bool,
    pub cycle_node: Option<String>,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomExplosionPreviewReadModel {
    pub material_id: String,
    pub quantity: i32,
    pub variant_code: Option<String>,
    pub items: Vec<BomExplosionItemReadModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomExplosionItemReadModel {
    pub bom_level: i32,
    pub parent_material_id: String,
    pub component_material_id: String,
    pub component_name: String,
    pub unit_qty: Decimal,
    pub required_qty: Decimal,
    pub available_qty: i32,
    pub shortage_qty: Decimal,
    pub is_critical: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomLifecycleReadModel {
    pub success: bool,
    pub bom_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomComponentCountReadModel {
    pub success: bool,
    pub bom_id: String,
    pub component_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCenterReadModel {
    pub work_center_id: String,
    pub work_center_name: String,
    pub location: Option<String>,
    pub capacity_per_day: Option<i32>,
    pub efficiency: Option<Decimal>,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionCharacteristicReadModel {
    pub char_id: String,
    pub char_name: String,
    pub material_type: Option<String>,
    pub inspection_type: Option<String>,
    pub method: Option<String>,
    pub standard: Option<String>,
    pub unit: Option<String>,
    pub lower_limit: Option<Decimal>,
    pub upper_limit: Option<Decimal>,
    pub is_critical: Option<bool>,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectCodeReadModel {
    pub defect_code: String,
    pub defect_name: String,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub created_at: Option<OffsetDateTime>,
}
