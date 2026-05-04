use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct BomExplosionPreviewRequest {
    pub variant_code: Option<String>,
    pub finished_material_id: String,
    pub quantity: i32,

    #[serde(default)]
    pub merge_components: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductionOrderRequest {
    pub variant_code: String,
    pub finished_material_id: String,
    pub bom_id: String,
    pub planned_qty: i32,
    pub work_center_id: String,
    pub planned_start_date: Option<NaiveDate>,
    pub planned_end_date: Option<NaiveDate>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct CompleteProductionOrderRequest {
    pub completed_qty: i32,
    pub finished_batch_number: String,
    pub finished_to_bin: String,
    pub posting_date: Option<DateTime<Utc>>,

    #[serde(default = "default_pick_strategy")]
    pub pick_strategy: String,

    pub remark: Option<String>,
}

fn default_pick_strategy() -> String {
    "FEFO".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListProductionOrdersRequest {
    pub status: Option<String>,
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub work_center_id: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListProductionVariancesRequest {
    pub order_id: Option<String>,
    pub variant_code: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub only_over_budget: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionApiMessage {
    pub module: &'static str,
    pub action: &'static str,
}