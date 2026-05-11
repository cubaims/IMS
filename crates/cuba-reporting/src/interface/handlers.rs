use super::dto::{
    BatchStockSummaryExportQuery, BatchStockSummaryReportQuery, BinStockSummaryExportQuery,
    BinStockSummaryReportQuery, CurrentStockExportQuery, CurrentStockReportQuery,
    DataConsistencyExportQuery, DataConsistencyReportQuery, InventoryValueExportQuery,
    InventoryValueReportQuery, LowStockAlertExportQuery, LowStockAlertReportQuery,
    MrpShortageExportQuery, MrpShortageReportQuery, QualityStatusExportQuery,
    QualityStatusReportQuery, RefreshReportsRequest, RefreshReportsResponse, ReportingResponse,
    StockByZoneExportQuery, StockByZoneReportQuery,
};
use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState, Page, PageQuery};
use serde_json::Value;

use crate::{
    application::{
        ExportReportUseCase, GetReportUseCase, RefreshMaterializedViewsCommand,
        RefreshMaterializedViewsUseCase,
    },
    domain::{
        BatchStockSummaryReportFilter, BinStockSummaryReportFilter, CurrentStockReportFilter,
        DataConsistencyReportFilter, ExportedReport, InventoryValueReportFilter,
        LowStockAlertReportFilter, MrpShortageReportFilter, QualityStatusReportFilter,
        ReportExportFormat, ReportExportRequest, ReportFilters, ReportQuery, ReportType,
        StockByZoneReportFilter,
    },
    infrastructure::PostgresReportingRepository,
};

pub async fn health(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse {
        module: "reporting",
        status: "ready",
    })))
}

pub async fn current_stock(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, current_stock_report_query(query)).await
}

pub async fn current_stock_export(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::CurrentStock,
        ReportFilters::CurrentStock(CurrentStockReportFilter {
            material_id: query.material_id,
            material_name: query.material_name,
            bin_code: query.bin_code,
            batch_number: query.batch_number,
            quality_status: query.quality_status,
            zone_code: query.zone_code,
            only_available: query.only_available.unwrap_or(false),
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn inventory_value(
    State(state): State<AppState>,
    Query(query): Query<InventoryValueReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, inventory_value_report_query(query)).await
}

pub async fn inventory_value_export(
    State(state): State<AppState>,
    Query(query): Query<InventoryValueExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::InventoryValue,
        ReportFilters::InventoryValue(InventoryValueReportFilter {
            material_id: query.material_id,
            material_type: query.material_type,
            only_positive_value: query.only_positive_value.unwrap_or(false),
            sort_by: query.sort_by,
            sort_order: query.sort_order,
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn quality_status(
    State(state): State<AppState>,
    Query(query): Query<QualityStatusReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, quality_status_report_query(query)).await
}

pub async fn quality_status_export(
    State(state): State<AppState>,
    Query(query): Query<QualityStatusExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::QualityStatus,
        ReportFilters::QualityStatus(QualityStatusReportFilter {
            material_id: query.material_id,
            quality_status: query.quality_status,
            batch_number: query.batch_number,
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn mrp_shortage(
    State(state): State<AppState>,
    Query(query): Query<MrpShortageReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, mrp_shortage_report_query(query)).await
}

pub async fn mrp_shortage_export(
    State(state): State<AppState>,
    Query(query): Query<MrpShortageExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::MrpShortage,
        ReportFilters::MrpShortage(MrpShortageReportFilter {
            run_id: query.run_id,
            material_id: query.material_id,
            suggestion_type: query.suggestion_type,
            only_open: query.only_open.unwrap_or(false),
            date_from: query.date_from,
            date_to: query.date_to,
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn low_stock_alert(
    State(state): State<AppState>,
    Query(query): Query<LowStockAlertReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, low_stock_alert_report_query(query)).await
}

pub async fn low_stock_alert_export(
    State(state): State<AppState>,
    Query(query): Query<LowStockAlertExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::LowStockAlert,
        ReportFilters::LowStockAlert(LowStockAlertReportFilter {
            material_id: query.material_id,
            material_type: query.material_type,
            severity: query.severity,
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn stock_by_zone(
    State(state): State<AppState>,
    Query(query): Query<StockByZoneReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, stock_by_zone_report_query(query)).await
}

pub async fn stock_by_zone_export(
    State(state): State<AppState>,
    Query(query): Query<StockByZoneExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::StockByZone,
        ReportFilters::StockByZone(StockByZoneReportFilter {
            material_id: query.material_id,
            material_type: query.material_type,
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn bin_stock_summary(
    State(state): State<AppState>,
    Query(query): Query<BinStockSummaryReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, bin_stock_summary_report_query(query)).await
}

pub async fn bin_stock_summary_export(
    State(state): State<AppState>,
    Query(query): Query<BinStockSummaryExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::BinStockSummary,
        ReportFilters::BinStockSummary(BinStockSummaryReportFilter {
            bin_code: query.bin_code,
            zone_code: query.zone_code,
            only_over_capacity: query.only_over_capacity.unwrap_or(false),
            only_occupied: query.only_occupied.unwrap_or(false),
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn batch_stock_summary(
    State(state): State<AppState>,
    Query(query): Query<BatchStockSummaryReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, batch_stock_summary_report_query(query)).await
}

pub async fn batch_stock_summary_export(
    State(state): State<AppState>,
    Query(query): Query<BatchStockSummaryExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::BatchStockSummary,
        ReportFilters::BatchStockSummary(BatchStockSummaryReportFilter {
            material_id: query.material_id,
            batch_number: query.batch_number,
            quality_status: query.quality_status,
            only_expiring: query.only_expiring.unwrap_or(false),
            only_expired: query.only_expired.unwrap_or(false),
            expiry_date_before: query.expiry_date_before,
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn data_consistency(
    State(state): State<AppState>,
    Query(query): Query<DataConsistencyReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    query_report(state, data_consistency_report_query(query)).await
}

pub async fn data_consistency_export(
    State(state): State<AppState>,
    Query(query): Query<DataConsistencyExportQuery>,
) -> AppResult<Response> {
    export_report(
        state,
        ReportType::DataConsistency,
        ReportFilters::DataConsistency(DataConsistencyReportFilter {
            material_id: query.material_id,
            only_inconsistent: query.only_inconsistent.unwrap_or(false),
        }),
        query.format,
        query.include_headers,
    )
    .await
}

pub async fn refresh(
    State(state): State<AppState>,
    body: Bytes,
) -> AppResult<Json<ApiResponse<RefreshReportsResponse>>> {
    let request = if body.is_empty() {
        RefreshReportsRequest {
            mode: None,
            concurrently: None,
            remark: None,
        }
    } else {
        serde_json::from_slice::<RefreshReportsRequest>(&body).map_err(|err| {
            AppError::business("REPORT_QUERY_INVALID", format!("刷新请求 JSON 无效: {err}"))
        })?
    };

    let mode = normalize_optional_string(request.mode).unwrap_or_else(|| "all".to_string());

    if mode != "all" {
        return Err(AppError::business(
            "REPORT_QUERY_INVALID",
            "MVP 阶段仅支持 mode=all",
        ));
    }

    if request.concurrently == Some(false) {
        return Err(AppError::business(
            "REPORT_QUERY_INVALID",
            "当前刷新函数固定使用 CONCURRENTLY，MVP 阶段仅支持 concurrently=true",
        ));
    }

    let command = RefreshMaterializedViewsCommand {
        mode,
        concurrently: true,
        remark: normalize_optional_string(request.remark),
    };

    let repository = PostgresReportingRepository::new(state);
    let use_case = RefreshMaterializedViewsUseCase::new(repository);
    let result = use_case.execute(command).await?;

    Ok(Json(ApiResponse::ok(RefreshReportsResponse {
        refreshed: result.refreshed,
        refreshed_at: result.refreshed_at,
        mode: result.mode,
        concurrently: result.concurrently,
        views: result.views,
        remark: result.remark,
    })))
}

async fn query_report(
    state: AppState,
    query: ReportQuery,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let repository = PostgresReportingRepository::new(state);
    let use_case = GetReportUseCase::new(repository);
    let page = use_case.execute(query).await?;
    Ok(Json(ApiResponse::ok(page)))
}

async fn export_report(
    state: AppState,
    report_type: ReportType,
    filters: ReportFilters,
    format: Option<String>,
    include_headers: Option<bool>,
) -> AppResult<Response> {
    let export_format = parse_export_format(format)?;
    let repository = PostgresReportingRepository::new(state);
    let use_case = ExportReportUseCase::new(repository);

    let exported = use_case
        .execute(ReportExportRequest {
            report_type,
            filters,
            format: export_format,
            include_headers: include_headers.unwrap_or(true),
        })
        .await?;

    csv_response(exported)
}

fn current_stock_report_query(query: CurrentStockReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::CurrentStock,
        filters: ReportFilters::CurrentStock(CurrentStockReportFilter {
            material_id: query.material_id,
            material_name: query.material_name,
            bin_code: query.bin_code,
            batch_number: query.batch_number,
            quality_status: query.quality_status,
            zone_code: query.zone_code,
            only_available: query.only_available.unwrap_or(false),
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn inventory_value_report_query(query: InventoryValueReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::InventoryValue,
        filters: ReportFilters::InventoryValue(InventoryValueReportFilter {
            material_id: query.material_id,
            material_type: query.material_type,
            only_positive_value: query.only_positive_value.unwrap_or(false),
            sort_by: query.sort_by,
            sort_order: query.sort_order,
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn quality_status_report_query(query: QualityStatusReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::QualityStatus,
        filters: ReportFilters::QualityStatus(QualityStatusReportFilter {
            material_id: query.material_id,
            quality_status: query.quality_status,
            batch_number: query.batch_number,
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn mrp_shortage_report_query(query: MrpShortageReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::MrpShortage,
        filters: ReportFilters::MrpShortage(MrpShortageReportFilter {
            run_id: query.run_id,
            material_id: query.material_id,
            suggestion_type: query.suggestion_type,
            only_open: query.only_open.unwrap_or(false),
            date_from: query.date_from,
            date_to: query.date_to,
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn low_stock_alert_report_query(query: LowStockAlertReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::LowStockAlert,
        filters: ReportFilters::LowStockAlert(LowStockAlertReportFilter {
            material_id: query.material_id,
            material_type: query.material_type,
            severity: query.severity,
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn stock_by_zone_report_query(query: StockByZoneReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::StockByZone,
        filters: ReportFilters::StockByZone(StockByZoneReportFilter {
            material_id: query.material_id,
            material_type: query.material_type,
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn bin_stock_summary_report_query(query: BinStockSummaryReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::BinStockSummary,
        filters: ReportFilters::BinStockSummary(BinStockSummaryReportFilter {
            bin_code: query.bin_code,
            zone_code: query.zone_code,
            only_over_capacity: query.only_over_capacity.unwrap_or(false),
            only_occupied: query.only_occupied.unwrap_or(false),
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn batch_stock_summary_report_query(query: BatchStockSummaryReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::BatchStockSummary,
        filters: ReportFilters::BatchStockSummary(BatchStockSummaryReportFilter {
            material_id: query.material_id,
            batch_number: query.batch_number,
            quality_status: query.quality_status,
            only_expiring: query.only_expiring.unwrap_or(false),
            only_expired: query.only_expired.unwrap_or(false),
            expiry_date_before: query.expiry_date_before,
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn data_consistency_report_query(query: DataConsistencyReportQuery) -> ReportQuery {
    ReportQuery {
        report_type: ReportType::DataConsistency,
        filters: ReportFilters::DataConsistency(DataConsistencyReportFilter {
            material_id: query.material_id,
            only_inconsistent: query.only_inconsistent.unwrap_or(false),
        }),
        page: page_query(query.page, query.page_size),
    }
}

fn page_query(page: Option<u64>, page_size: Option<u64>) -> PageQuery {
    PageQuery {
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    }
}

fn parse_export_format(format: Option<String>) -> AppResult<ReportExportFormat> {
    let format = normalize_optional_string(format).unwrap_or_else(|| "csv".to_string());

    if format.eq_ignore_ascii_case("csv") {
        return Ok(ReportExportFormat::Csv);
    }

    Err(AppError::business(
        "REPORT_FORMAT_UNSUPPORTED",
        "报表导出 MVP 仅支持 csv",
    ))
}

fn csv_response(exported: ExportedReport) -> AppResult<Response> {
    let content_type = HeaderValue::from_str(&exported.content_type).map_err(|err| {
        AppError::business("REPORT_EXPORT_FAILED", format!("生成导出响应头失败: {err}"))
    })?;
    let disposition =
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", exported.filename)).map_err(
            |err| AppError::business("REPORT_EXPORT_FAILED", format!("生成导出响应头失败: {err}")),
        )?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        exported.body,
    )
        .into_response())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
