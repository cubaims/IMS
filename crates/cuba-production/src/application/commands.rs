use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateProductionOrderCommand {
    pub variant_code: String,
    pub finished_material_id: String,
    pub bom_id: String,

    #[validate(range(min = 1))]
    pub planned_qty: i32,

    pub work_center_id: String,
    pub planned_start_date: Option<NaiveDate>,
    pub planned_end_date: Option<NaiveDate>,
    pub remark: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ReleaseProductionOrderCommand {
    pub order_id: String,
    pub remark: Option<String>,
    pub operator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CompleteProductionOrderCommand {
    pub order_id: String,

    #[validate(range(min = 1))]
    pub completed_qty: i32,

    pub finished_batch_number: String,
    pub finished_to_bin: String,
    pub posting_date: Option<DateTime<Utc>>,
    pub pick_strategy: Option<String>,
    pub remark: Option<String>,
    pub operator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BomExplosionCommand {
    pub variant_code: Option<String>,
    pub finished_material_id: String,

    #[validate(range(min = 1))]
    pub quantity: i32,

    pub merge_components: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOrderQuery {
    pub order_id: Option<String>,
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionVarianceQuery {
    pub order_id: Option<String>,
    pub variant_code: Option<String>,
    pub only_over_budget: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}