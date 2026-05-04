use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateMaterialCommand {
    #[validate(length(min = 1, max = 20))]
    pub material_id: String,

    #[validate(length(min = 1, max = 100))]
    pub material_name: String,

    #[validate(length(min = 1))]
    pub material_type: String,

    #[validate(length(min = 1, max = 10))]
    pub base_unit: String,

    #[validate(length(min = 1, max = 10))]
    pub default_zone: String,

    #[validate(range(min = 0))]
    pub safety_stock: i32,

    #[validate(range(min = 0))]
    pub reorder_point: i32,

    pub standard_price: Decimal,

    pub map_price: Decimal,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateMaterialCommand {
    #[validate(length(min = 1, max = 100))]
    pub material_name: Option<String>,

    #[validate(length(min = 1, max = 10))]
    pub base_unit: Option<String>,

    #[validate(length(min = 1, max = 10))]
    pub default_zone: Option<String>,

    #[validate(range(min = 0))]
    pub safety_stock: Option<i32>,

    #[validate(range(min = 0))]
    pub reorder_point: Option<i32>,

    pub standard_price: Option<Decimal>,

    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateStorageBinCommand {
    #[validate(length(min = 1, max = 20))]
    pub bin_code: String,

    #[validate(length(min = 1, max = 10))]
    pub zone: String,

    #[validate(length(min = 1, max = 20))]
    pub bin_type: String,

    #[validate(range(min = 1))]
    pub capacity: i32,

    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateStorageBinCommand {
    #[validate(length(min = 1, max = 10))]
    pub zone: Option<String>,

    #[validate(length(min = 1, max = 20))]
    pub bin_type: Option<String>,

    #[validate(range(min = 1))]
    pub capacity: Option<i32>,

    pub status: Option<String>,

    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSupplierCommand {
    #[validate(length(min = 1, max = 20))]
    pub supplier_id: String,

    #[validate(length(min = 1, max = 100))]
    pub supplier_name: String,

    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub quality_rating: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateSupplierCommand {
    #[validate(length(min = 1, max = 100))]
    pub supplier_name: Option<String>,

    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub quality_rating: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCustomerCommand {
    #[validate(length(min = 1, max = 20))]
    pub customer_id: String,

    #[validate(length(min = 1, max = 100))]
    pub customer_name: String,

    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub credit_limit: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateCustomerCommand {
    #[validate(length(min = 1, max = 100))]
    pub customer_name: Option<String>,

    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub credit_limit: Option<Decimal>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateMaterialSupplierCommand {
    #[validate(length(min = 1, max = 20))]
    pub material_id: String,

    #[validate(length(min = 1, max = 20))]
    pub supplier_id: String,

    pub is_primary: Option<bool>,
    pub supplier_material_code: Option<String>,
    pub purchase_price: Option<Decimal>,
    pub currency: Option<String>,

    #[validate(range(min = 0))]
    pub lead_time_days: Option<i32>,

    #[validate(range(min = 1))]
    pub moq: Option<i32>,

    pub quality_rating: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateMaterialSupplierCommand {
    pub is_primary: Option<bool>,
    pub supplier_material_code: Option<String>,
    pub purchase_price: Option<Decimal>,
    pub currency: Option<String>,

    #[validate(range(min = 0))]
    pub lead_time_days: Option<i32>,

    #[validate(range(min = 1))]
    pub moq: Option<i32>,

    pub quality_rating: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateProductVariantCommand {
    #[validate(length(min = 1, max = 20))]
    pub variant_code: String,

    #[validate(length(min = 1, max = 100))]
    pub variant_name: String,

    #[validate(length(min = 1, max = 20))]
    pub base_material_id: String,

    pub bom_id: Option<String>,

    pub standard_cost: Decimal,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateProductVariantCommand {
    #[validate(length(min = 1, max = 100))]
    pub variant_name: Option<String>,

    pub bom_id: Option<String>,

    pub standard_cost: Option<Decimal>,

    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBomHeaderCommand {
    #[validate(length(min = 1, max = 30))]
    pub bom_id: String,

    #[validate(length(min = 1, max = 100))]
    pub bom_name: String,

    #[validate(length(min = 1, max = 20))]
    pub parent_material_id: String,

    pub variant_code: Option<String>,

    #[validate(length(min = 1, max = 10))]
    pub version: String,

    pub base_quantity: Option<Decimal>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateBomHeaderCommand {
    #[validate(length(min = 1, max = 100))]
    pub bom_name: Option<String>,

    pub variant_code: Option<String>,

    #[validate(length(min = 1, max = 10))]
    pub version: Option<String>,

    pub base_quantity: Option<Decimal>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBomComponentCommand {
    #[validate(length(min = 1, max = 30))]
    pub bom_id: String,

    #[validate(length(min = 1, max = 20))]
    pub parent_material_id: String,

    #[validate(length(min = 1, max = 20))]
    pub component_material_id: String,

    pub quantity: Decimal,

    #[validate(length(min = 1, max = 10))]
    pub unit: String,

    pub bom_level: Option<i32>,
    pub scrap_rate: Option<Decimal>,
    pub is_critical: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateBomComponentCommand {
    pub quantity: Option<Decimal>,

    #[validate(length(min = 1, max = 10))]
    pub unit: Option<String>,

    pub bom_level: Option<i32>,
    pub scrap_rate: Option<Decimal>,
    pub is_critical: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateWorkCenterCommand {
    #[validate(length(min = 1, max = 20))]
    pub work_center_id: String,

    #[validate(length(min = 1, max = 100))]
    pub work_center_name: String,

    pub location: Option<String>,

    #[validate(range(min = 1))]
    pub capacity_per_day: Option<i32>,

    pub efficiency: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateWorkCenterCommand {
    #[validate(length(min = 1, max = 100))]
    pub work_center_name: Option<String>,

    pub location: Option<String>,

    #[validate(range(min = 1))]
    pub capacity_per_day: Option<i32>,

    pub efficiency: Option<Decimal>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateInspectionCharCommand {
    #[validate(length(min = 1, max = 30))]
    pub char_id: String,

    #[validate(length(min = 1, max = 100))]
    pub char_name: String,

    pub material_type: Option<String>,
    pub inspection_type: Option<String>,
    pub method: Option<String>,
    pub standard: Option<String>,
    pub unit: Option<String>,
    pub lower_limit: Option<Decimal>,
    pub upper_limit: Option<Decimal>,
    pub is_critical: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateInspectionCharCommand {
    #[validate(length(min = 1, max = 100))]
    pub char_name: Option<String>,

    pub material_type: Option<String>,
    pub inspection_type: Option<String>,
    pub method: Option<String>,
    pub standard: Option<String>,
    pub unit: Option<String>,
    pub lower_limit: Option<Decimal>,
    pub upper_limit: Option<Decimal>,
    pub is_critical: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateDefectCodeCommand {
    #[validate(length(min = 1, max = 20))]
    pub defect_code: String,

    #[validate(length(min = 1, max = 100))]
    pub defect_name: String,

    pub category: Option<String>,
    pub severity: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateDefectCodeCommand {
    #[validate(length(min = 1, max = 100))]
    pub defect_name: Option<String>,

    pub category: Option<String>,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MasterDataQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl MasterDataQuery {
    pub fn limit(&self) -> i64 {
        self.page_size.unwrap_or(20).clamp(1, 200) as i64
    }

    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        ((page - 1) as i64) * self.limit()
    }
}
