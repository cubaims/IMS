use crate::domain::{MrpRunId, MrpRunStatus, MrpSuggestionStatus, MrpSuggestionType};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::Date;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpRequest {
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpResponse {
    pub module: &'static str,
    pub status: &'static str,
}

/// 运行 MRP 请求。
///
/// MVP 先按产品变体运行，因为当前 wms.fn_run_mrp() 需要 variant_code。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMrpRequest {
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub demand_qty: Decimal,
    pub demand_date: Date,
    pub remark: Option<String>,
}

/// 运行 MRP 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMrpResponse {
    pub run_id: MrpRunId,
    pub status: MrpRunStatus,
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub demand_qty: Decimal,
    pub demand_date: Date,
    pub suggestion_count: u64,
    pub shortage_count: u64,
}

/// MRP 运行记录查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpRunsQueryRequest {
    pub variant_code: Option<String>,
    pub finished_material_id: Option<String>,
    pub status: Option<MrpRunStatus>,
    pub date_from: Option<time::OffsetDateTime>,
    pub date_to: Option<time::OffsetDateTime>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// MRP 建议查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpSuggestionsQueryRequest {
    pub run_id: Option<String>,
    pub material_id: Option<String>,
    pub suggestion_type: Option<MrpSuggestionType>,
    pub status: Option<MrpSuggestionStatus>,
    pub date_from: Option<time::OffsetDateTime>,
    pub date_to: Option<time::OffsetDateTime>,
    pub only_shortage: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// MRP 建议导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpSuggestionsExportQueryRequest {
    pub run_id: Option<String>,
    pub material_id: Option<String>,
    pub suggestion_type: Option<MrpSuggestionType>,
    pub status: Option<MrpSuggestionStatus>,
    pub date_from: Option<time::OffsetDateTime>,
    pub date_to: Option<time::OffsetDateTime>,
    pub only_shortage: Option<bool>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}

/// 确认 MRP 建议请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmMrpSuggestionRequest {
    pub remark: Option<String>,
}

/// 确认 MRP 建议响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmMrpSuggestionResponse {
    pub suggestion_id: crate::domain::MrpSuggestionId,
    pub status: crate::domain::MrpSuggestionStatus,
}

/// 取消 MRP 建议请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelMrpSuggestionRequest {
    pub reason: String,
}

/// 取消 MRP 建议响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelMrpSuggestionResponse {
    pub suggestion_id: crate::domain::MrpSuggestionId,
    pub status: crate::domain::MrpSuggestionStatus,
}
