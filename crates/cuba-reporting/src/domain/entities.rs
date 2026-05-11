use cuba_shared::{Page, PageQuery};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingSummary {
    pub code: String,
    pub name: String,
    pub status: String,
}

/// Phase 9 支持的报表类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportType {
    CurrentStock,
    InventoryValue,
    QualityStatus,
    MrpShortage,
    LowStockAlert,
    StockByZone,
    BinStockSummary,
    BatchStockSummary,
    DataConsistency,
}

impl ReportType {
    pub fn view_name(self) -> &'static str {
        match self {
            Self::CurrentStock => "rpt_current_stock",
            Self::InventoryValue => "rpt_inventory_value",
            Self::QualityStatus => "rpt_quality_status",
            Self::MrpShortage => "rpt_mrp_shortage",
            Self::LowStockAlert => "rpt_low_stock_alert",
            Self::StockByZone => "rpt_stock_by_zone",
            Self::BinStockSummary => "rpt_bin_stock_summary",
            Self::BatchStockSummary => "rpt_batch_stock_summary",
            Self::DataConsistency => "rpt_data_consistency_check",
        }
    }
}

/// 报表导出格式。MVP 只实现 CSV。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportExportFormat {
    Csv,
}

pub type ReportPage = Page<Value>;

/// 报表查询请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportQuery {
    pub report_type: ReportType,
    pub filters: ReportFilters,
    pub page: PageQuery,
}

/// 报表导出请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportExportRequest {
    pub report_type: ReportType,
    pub filters: ReportFilters,
    pub format: ReportExportFormat,
    pub include_headers: bool,
}

/// 已导出的报表文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedReport {
    pub filename: String,
    pub content_type: String,
    pub body: String,
}

/// 报表过滤条件集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "report_type", content = "filters", rename_all = "kebab-case")]
pub enum ReportFilters {
    CurrentStock(CurrentStockReportFilter),
    InventoryValue(InventoryValueReportFilter),
    QualityStatus(QualityStatusReportFilter),
    MrpShortage(MrpShortageReportFilter),
    LowStockAlert(LowStockAlertReportFilter),
    StockByZone(StockByZoneReportFilter),
    BinStockSummary(BinStockSummaryReportFilter),
    BatchStockSummary(BatchStockSummaryReportFilter),
    DataConsistency(DataConsistencyReportFilter),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentStockReportFilter {
    pub material_id: Option<String>,
    pub material_name: Option<String>,
    pub bin_code: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub zone_code: Option<String>,
    pub only_available: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryValueReportFilter {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub only_positive_value: bool,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityStatusReportFilter {
    pub material_id: Option<String>,
    pub quality_status: Option<String>,
    pub batch_number: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MrpShortageReportFilter {
    pub run_id: Option<String>,
    pub material_id: Option<String>,
    pub suggestion_type: Option<String>,
    pub only_open: bool,
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LowStockAlertReportFilter {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockByZoneReportFilter {
    pub material_id: Option<String>,
    pub material_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinStockSummaryReportFilter {
    pub bin_code: Option<String>,
    pub zone_code: Option<String>,
    pub only_over_capacity: bool,
    pub only_occupied: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchStockSummaryReportFilter {
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub only_expiring: bool,
    pub only_expired: bool,
    pub expiry_date_before: Option<Date>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataConsistencyReportFilter {
    pub material_id: Option<String>,
    pub only_inconsistent: bool,
}

/// 数据一致性检查行。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConsistencyRow {
    pub material_id: String,
    pub material_stock: Decimal,
    pub bin_stock: Decimal,
    pub batch_stock: Decimal,
    pub is_consistent: bool,
    pub difference_material_vs_bin: Decimal,
    pub difference_material_vs_batch: Decimal,
}

/// 物化视图刷新结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRefreshResult {
    pub refreshed: bool,
    pub refreshed_at: OffsetDateTime,
    pub mode: String,
    pub concurrently: bool,
    pub views: Vec<String>,
    pub remark: Option<String>,
}
