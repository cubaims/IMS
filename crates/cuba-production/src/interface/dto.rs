use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateProductionOrderRequest {
    pub variant_code: String,
    pub finished_material_id: String,
    pub bom_id: String,

    #[validate(range(min = 1))]
    pub planned_qty: i32,

    pub work_center_id: String,
    pub planned_start_date: Option<Date>,
    pub planned_end_date: Option<Date>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseProductionOrderRequest {
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelProductionOrderRequest {
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloseProductionOrderRequest {
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CompleteProductionOrderRequest {
    #[validate(range(min = 1))]
    pub completed_qty: i32,

    pub finished_batch_number: String,
    pub finished_to_bin: String,
    pub posting_date: Option<OffsetDateTime>,
    pub pick_strategy: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct BomExplosionPreviewRequest {
    pub variant_code: Option<String>,
    pub finished_material_id: String,

    #[validate(range(min = 1))]
    pub quantity: i32,

    pub merge_components: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductionOrderListQuery {
    pub order_id: Option<String>,
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductionVarianceListQuery {
    pub order_id: Option<String>,
    pub variant_code: Option<String>,
    pub only_over_budget: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedProductionOrderResponse {
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionActionResponse {
    pub order_id: String,
    pub status: String,
}
