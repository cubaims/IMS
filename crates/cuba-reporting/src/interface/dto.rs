use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingRequest {
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingResponse {
    pub module: &'static str,
    pub status: &'static str,
}

/// 当前库存报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStockReportQuery {
    pub material_id: Option<String>,
    pub material_name: Option<String>,
    pub bin_code: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub zone_code: Option<String>,
    pub only_available: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 库存价值报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryValueReportQuery {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub only_positive_value: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 质量状态报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStatusReportQuery {
    pub material_id: Option<String>,
    pub quality_status: Option<String>,
    pub batch_number: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// MRP 短缺报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpShortageReportQuery {
    pub run_id: Option<String>,
    pub material_id: Option<String>,
    pub suggestion_type: Option<String>,
    pub only_open: Option<bool>,
    pub date_from: Option<time::OffsetDateTime>,
    pub date_to: Option<time::OffsetDateTime>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 低库存预警报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockAlertReportQuery {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub severity: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 数据一致性检查报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConsistencyReportQuery {
    pub material_id: Option<String>,
    pub only_inconsistent: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 区域库存矩阵报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockByZoneReportQuery {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 货位库存汇总报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinStockSummaryReportQuery {
    pub bin_code: Option<String>,
    pub zone_code: Option<String>,
    pub only_over_capacity: Option<bool>,
    pub only_occupied: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 批次库存汇总报表查询参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStockSummaryReportQuery {
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub only_expiring: Option<bool>,
    pub only_expired: Option<bool>,
    pub expiry_date_before: Option<time::Date>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 当前库存报表导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStockExportQuery {
    pub material_id: Option<String>,
    pub material_name: Option<String>,
    pub bin_code: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub zone_code: Option<String>,
    pub only_available: Option<bool>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}

/// 库存价值报表导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryValueExportQuery {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub only_positive_value: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}

/// MRP 短缺报表导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpShortageExportQuery {
    pub run_id: Option<String>,
    pub material_id: Option<String>,
    pub suggestion_type: Option<String>,
    pub only_open: Option<bool>,
    pub date_from: Option<time::OffsetDateTime>,
    pub date_to: Option<time::OffsetDateTime>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}

/// 低库存预警报表导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockAlertExportQuery {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub severity: Option<String>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}

/// 批次库存汇总报表导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStockSummaryExportQuery {
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub only_expiring: Option<bool>,
    pub only_expired: Option<bool>,
    pub expiry_date_before: Option<time::Date>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}

/// 数据一致性检查导出参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConsistencyExportQuery {
    pub material_id: Option<String>,
    pub only_inconsistent: Option<bool>,

    /// MVP 仅支持 csv。
    pub format: Option<String>,

    /// 是否包含表头，默认 true。
    pub include_headers: Option<bool>,
}
