use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PreviewBomExplosionCommand {
    pub variant_code: Option<String>,

    #[validate(length(min = 1))]
    pub finished_material_id: String,

    #[validate(range(min = 1))]
    pub quantity: i32,

    #[serde(default)]
    pub merge_components: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateProductionOrderCommand {
    pub variant_code: String,

    #[validate(length(min = 1))]
    pub finished_material_id: String,

    #[validate(length(min = 1))]
    pub bom_id: String,

    #[validate(range(min = 1))]
    pub planned_qty: i32,

    #[validate(length(min = 1))]
    pub work_center_id: String,

    pub planned_start_date: Option<NaiveDate>,

    pub planned_end_date: Option<NaiveDate>,

    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ReleaseProductionOrderCommand {
    pub order_id: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CompleteProductionOrderCommand {
    pub order_id: String,

    #[validate(range(min = 1))]
    pub completed_qty: i32,

    #[validate(length(min = 1))]
    pub finished_batch_number: String,

    #[validate(length(min = 1))]
    pub finished_to_bin: String,

    pub posting_date: Option<DateTime<Utc>>,

    #[serde(default = "default_pick_strategy")]
    pub pick_strategy: String,

    pub remark: Option<String>,
}

fn default_pick_strategy() -> String {
    "FEFO".to_string()
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ListProductionOrdersQuery {
    pub status: Option<String>,
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub work_center_id: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ListProductionVariancesQuery {
    pub order_id: Option<String>,
    pub variant_code: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub only_over_budget: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProductionOrderResult {
    pub order_id: String,
    pub status: String,
    pub variant_code: String,
    pub finished_material_id: String,
    pub planned_qty: i32,
    pub component_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseProductionOrderResult {
    pub order_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionCompleteAppResult {
    pub order_id: String,
    pub status: String,
    pub completed_qty: i32,
    pub finished_transaction: Option<ProductionTransactionDto>,
    pub component_transactions: Vec<ProductionTransactionDto>,
    pub genealogy_count: i64,
    pub variance_id: Option<i64>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionTransactionDto {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
}