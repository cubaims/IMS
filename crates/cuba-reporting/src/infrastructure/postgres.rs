use crate::application::{
    MaterializedViewRepository, RefreshMaterializedViewsCommand, ReportExportRepository,
    ReportingRepository,
};
use crate::domain::{
    BatchStockSummaryReportFilter, BinStockSummaryReportFilter, CurrentStockReportFilter,
    DataConsistencyReportFilter, ExportedReport, InventoryValueReportFilter,
    LowStockAlertReportFilter, MrpShortageReportFilter, QualityStatusReportFilter,
    ReportExportFormat, ReportExportRequest, ReportFilters, ReportPage, ReportQuery,
    ReportRefreshResult, ReportType, StockByZoneReportFilter,
};
use async_trait::async_trait;
use cuba_shared::{AppError, AppResult, AppState, Page, map_reporting_db_error};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::HashSet;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct PostgresReportingRepository {
    state: AppState,
}

impl PostgresReportingRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ReportingRepository for PostgresReportingRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }

    async fn query_report(&self, query: ReportQuery) -> AppResult<ReportPage> {
        query_report(&self.state.db_pool, query).await
    }
}

#[async_trait]
impl MaterializedViewRepository for PostgresReportingRepository {
    async fn refresh_all(
        &self,
        command: RefreshMaterializedViewsCommand,
    ) -> AppResult<ReportRefreshResult> {
        sqlx::query("SELECT rpt.refresh_all_materialized_views()")
            .execute(&self.state.db_pool)
            .await
            .map_err(|err| match map_reporting_db_error(err) {
                AppError::Business {
                    code: "REPORT_QUERY_FAILED",
                    ..
                } => AppError::business("MATERIALIZED_VIEW_REFRESH_FAILED", "刷新报表物化视图失败"),
                mapped => mapped,
            })?;

        Ok(ReportRefreshResult {
            refreshed: true,
            refreshed_at: OffsetDateTime::now_utc(),
            mode: command.mode,
            concurrently: command.concurrently,
            views: vec![
                "rpt_current_stock".to_string(),
                "rpt_inventory_value".to_string(),
                "rpt_quality_status".to_string(),
                "rpt_mrp_shortage".to_string(),
                "rpt_low_stock_alert".to_string(),
                "rpt_stock_by_zone".to_string(),
                "rpt_bin_stock_summary".to_string(),
                "rpt_batch_stock_summary".to_string(),
            ],
            remark: command.remark,
        })
    }
}

#[async_trait]
impl ReportExportRepository for PostgresReportingRepository {
    async fn export_report(&self, request: ReportExportRequest) -> AppResult<ExportedReport> {
        if request.format != ReportExportFormat::Csv {
            return Err(AppError::business(
                "REPORT_FORMAT_UNSUPPORTED",
                "报表导出 MVP 仅支持 csv",
            ));
        }

        let filename = format!("{}.csv", report_slug(request.report_type));
        let mut query = ReportQuery {
            report_type: request.report_type,
            filters: request.filters,
            page: cuba_shared::PageQuery {
                page: 1,
                page_size: 200,
            },
        };
        let mut all_items = Vec::new();

        loop {
            let page = query_report(&self.state.db_pool, query.clone()).await?;
            let total = page.total;
            let item_count = page.items.len();

            all_items.extend(page.items);

            if all_items.len() as u64 >= total || item_count == 0 {
                break;
            }

            query.page.page = query.page.page.saturating_add(1);
        }

        Ok(ExportedReport {
            filename,
            content_type: "text/csv; charset=utf-8".to_string(),
            body: json_rows_to_csv(&all_items, request.include_headers),
        })
    }
}

async fn query_report(pool: &sqlx::PgPool, query: ReportQuery) -> AppResult<ReportPage> {
    validate_report_query(&query)?;

    let columns = load_report_columns(pool, "rpt", query.report_type.view_name()).await?;
    let page = query.page.page.max(1);
    let page_size = query.page.page_size.clamp(1, 200);
    let limit = page_size as i64;
    let offset = ((page - 1).saturating_mul(page_size)) as i64;

    let mut count_builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.");
    count_builder.push(query.report_type.view_name());
    count_builder.push(" WHERE 1 = 1");
    apply_report_filters(
        &mut count_builder,
        query.report_type,
        &query.filters,
        &columns,
    );

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(pool)
        .await
        .map_err(|err| map_report_db_error(query.report_type, "统计", err))?;

    let mut data_builder =
        QueryBuilder::<Postgres>::new("SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.");
    data_builder.push(query.report_type.view_name());
    data_builder.push(" WHERE 1 = 1");
    apply_report_filters(
        &mut data_builder,
        query.report_type,
        &query.filters,
        &columns,
    );
    data_builder.push(" ORDER BY ");
    data_builder.push(report_order_by(query.report_type, &query.filters, &columns));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(|err| map_report_db_error(query.report_type, "查询", err))?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| map_report_db_error(query.report_type, "结果转换", err))?;

    Ok(Page::new(items, total.max(0) as u64, page, page_size))
}

fn map_report_db_error(
    report_type: ReportType,
    operation: &'static str,
    err: sqlx::Error,
) -> AppError {
    match map_reporting_db_error(err) {
        AppError::Business {
            code: "REPORT_QUERY_FAILED",
            ..
        } => AppError::business(
            "REPORT_QUERY_FAILED",
            format!("{}{}失败", report_display_name(report_type), operation),
        ),
        mapped => mapped,
    }
}

fn validate_report_query(query: &ReportQuery) -> AppResult<()> {
    if let ReportFilters::MrpShortage(filter) = &query.filters {
        if let (Some(date_from), Some(date_to)) = (filter.date_from, filter.date_to) {
            if date_from >= date_to {
                return Err(AppError::business(
                    "REPORT_QUERY_INVALID",
                    "date_from 必须早于 date_to",
                ));
            }
        }
    }

    Ok(())
}

fn apply_report_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    report_type: ReportType,
    filters: &'a ReportFilters,
    columns: &HashSet<String>,
) {
    match (report_type, filters) {
        (ReportType::CurrentStock, ReportFilters::CurrentStock(filter)) => {
            apply_current_stock_filters(builder, filter);
        }
        (ReportType::InventoryValue, ReportFilters::InventoryValue(filter)) => {
            apply_inventory_value_filters(builder, filter);
        }
        (ReportType::QualityStatus, ReportFilters::QualityStatus(filter)) => {
            apply_quality_status_filters(builder, filter);
        }
        (ReportType::MrpShortage, ReportFilters::MrpShortage(filter)) => {
            apply_mrp_shortage_filters(builder, filter, columns);
        }
        (ReportType::LowStockAlert, ReportFilters::LowStockAlert(filter)) => {
            apply_low_stock_alert_filters(builder, filter, columns);
        }
        (ReportType::StockByZone, ReportFilters::StockByZone(filter)) => {
            apply_stock_by_zone_filters(builder, filter, columns);
        }
        (ReportType::BinStockSummary, ReportFilters::BinStockSummary(filter)) => {
            apply_bin_stock_summary_filters(builder, filter, columns);
        }
        (ReportType::BatchStockSummary, ReportFilters::BatchStockSummary(filter)) => {
            apply_batch_stock_summary_filters(builder, filter, columns);
        }
        (ReportType::DataConsistency, ReportFilters::DataConsistency(filter)) => {
            apply_data_consistency_filters(builder, filter, columns);
        }
        _ => {}
    }
}

fn report_order_by(
    report_type: ReportType,
    filters: &ReportFilters,
    columns: &HashSet<String>,
) -> &'static str {
    match report_type {
        ReportType::CurrentStock => "material_id, bin_code, batch_number",
        ReportType::InventoryValue => match filters {
            ReportFilters::InventoryValue(filter) => inventory_value_order_by(filter),
            _ => "total_map_value DESC, material_id ASC",
        },
        ReportType::QualityStatus => "material_id",
        ReportType::MrpShortage => first_existing_column(
            columns,
            &["priority", "material_id", "run_id"],
        )
        .map_or("1 ASC", |column| match column {
            "priority" => "priority ASC",
            "material_id" => "material_id ASC",
            "run_id" => "run_id ASC",
            _ => "1 ASC",
        }),
        ReportType::LowStockAlert => low_stock_alert_order_by(columns),
        ReportType::StockByZone => stock_by_zone_order_by(columns),
        ReportType::BinStockSummary => bin_stock_summary_order_by(columns),
        ReportType::BatchStockSummary => batch_stock_summary_order_by(columns),
        ReportType::DataConsistency => data_consistency_order_by(columns),
    }
}

fn apply_current_stock_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a CurrentStockReportFilter,
) {
    push_optional_equals_filter_unchecked(builder, "material_id", filter.material_id.clone());
    push_optional_like_filter_unchecked(builder, "material_name", filter.material_name.clone());
    push_optional_equals_filter_unchecked(builder, "bin_code", filter.bin_code.clone());
    push_optional_equals_filter_unchecked(builder, "batch_number", filter.batch_number.clone());
    push_optional_equals_filter_unchecked(builder, "quality_status", filter.quality_status.clone());
    push_optional_equals_filter_unchecked(builder, "zone", filter.zone_code.clone());

    if filter.only_available {
        builder.push(" AND COALESCE(qty, 0) > 0");
    }
}

fn apply_inventory_value_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a InventoryValueReportFilter,
) {
    push_optional_equals_filter_unchecked(builder, "material_id", filter.material_id.clone());
    push_optional_equals_filter_unchecked(builder, "material_type", filter.material_type.clone());

    if filter.only_positive_value {
        builder.push(" AND COALESCE(total_map_value, 0) > 0");
    }
}

fn inventory_value_order_by(filter: &InventoryValueReportFilter) -> &'static str {
    let sort_by = normalize_optional_string(filter.sort_by.clone())
        .unwrap_or_else(|| "total_map_value".to_string());

    let desc = normalize_optional_string(filter.sort_order.clone())
        .map(|v| v.eq_ignore_ascii_case("desc"))
        .unwrap_or(true);

    match (sort_by.as_str(), desc) {
        ("material_id", false) => "material_id ASC",
        ("material_id", true) => "material_id DESC",
        ("material_type", false) => "material_type ASC, material_id ASC",
        ("material_type", true) => "material_type DESC, material_id ASC",
        ("current_stock", false) => "current_stock ASC, material_id ASC",
        ("current_stock", true) => "current_stock DESC, material_id ASC",
        ("standard_cost", false) | ("standard_price", false) => {
            "standard_price ASC, material_id ASC"
        }
        ("standard_cost", true) | ("standard_price", true) => {
            "standard_price DESC, material_id ASC"
        }
        ("map_price", false) => "map_price ASC, material_id ASC",
        ("map_price", true) => "map_price DESC, material_id ASC",
        ("price_variance", false) => "price_variance ASC, material_id ASC",
        ("price_variance", true) => "price_variance DESC, material_id ASC",
        ("inventory_value", false) | ("value", false) | ("total_map_value", false) => {
            "total_map_value ASC, material_id ASC"
        }
        _ => "total_map_value DESC, material_id ASC",
    }
}

fn apply_quality_status_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a QualityStatusReportFilter,
) {
    push_optional_equals_filter_unchecked(builder, "material_id", filter.material_id.clone());

    if let Some(value) = normalize_optional_string(filter.quality_status.clone()) {
        match value.as_str() {
            "合格" | "PASS" | "PASSED" => {
                builder.push(" AND COALESCE(pass_count, 0) > 0");
            }
            "待检" | "PENDING" => {
                builder.push(" AND COALESCE(pending_count, 0) > 0");
            }
            "冻结" | "BLOCKED" | "FROZEN" => {
                builder.push(" AND COALESCE(blocked_count, 0) > 0");
            }
            _ => {}
        };
    }

    let _ = &filter.batch_number;
}

fn apply_mrp_shortage_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a MrpShortageReportFilter,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "run_id", filter.run_id.clone());
    push_optional_equals_filter(builder, columns, "material_id", filter.material_id.clone());

    if let Some(value) = normalize_optional_string(filter.suggestion_type.clone()) {
        if let Some(column) =
            first_existing_column(columns, &["suggestion_type", "suggested_order_type"])
        {
            builder.push(" AND ");
            builder.push(column);
            builder.push(" = ");
            builder.push_bind(value);
        }
    }

    if filter.only_open {
        if columns.contains("status") {
            builder.push(" AND status IN ('OPEN', 'NEW', '待处理', '新建')");
        } else if columns.contains("remarks") {
            builder.push(
                " AND COALESCE(NULLIF(CASE WHEN remarks ~ '^\\s*\\{' THEN remarks::jsonb ->> 'status' END, ''), 'OPEN') IN ('OPEN', 'NEW')",
            );
        }
    }

    if let Some(date_column) = first_existing_column(
        columns,
        &[
            "required_date",
            "suggested_date",
            "demand_date",
            "run_date",
            "created_at",
        ],
    ) {
        if let Some(date_from) = filter.date_from {
            builder.push(" AND ");
            builder.push(date_column);
            builder.push(" >= ");
            builder.push_bind(date_from);
        }

        if let Some(date_to) = filter.date_to {
            builder.push(" AND ");
            builder.push(date_column);
            builder.push(" < ");
            builder.push_bind(date_to);
        }
    }
}

fn apply_low_stock_alert_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a LowStockAlertReportFilter,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", filter.material_id.clone());
    push_optional_equals_filter(
        builder,
        columns,
        "material_type",
        filter.material_type.clone(),
    );
    push_optional_equals_filter_by_candidates(
        builder,
        columns,
        &["severity", "alert_level"],
        filter.severity.clone(),
    );
}

fn low_stock_alert_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("severity") && columns.contains("material_id") {
        "severity DESC, material_id ASC"
    } else if columns.contains("alert_level") && columns.contains("material_id") {
        "alert_level DESC, material_id ASC"
    } else if columns.contains("material_id") {
        "material_id ASC"
    } else {
        "1 ASC"
    }
}

fn apply_stock_by_zone_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a StockByZoneReportFilter,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", filter.material_id.clone());
    push_optional_equals_filter(
        builder,
        columns,
        "material_type",
        filter.material_type.clone(),
    );
}

fn stock_by_zone_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("material_type") && columns.contains("material_id") {
        "material_type ASC, material_id ASC"
    } else if columns.contains("material_id") {
        "material_id ASC"
    } else {
        "1 ASC"
    }
}

fn apply_bin_stock_summary_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a BinStockSummaryReportFilter,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "bin_code", filter.bin_code.clone());
    push_optional_equals_filter_by_candidates(
        builder,
        columns,
        &["zone_code", "zone"],
        filter.zone_code.clone(),
    );

    if filter.only_over_capacity {
        if columns.contains("capacity_usage_rate") {
            builder.push(" AND COALESCE(capacity_usage_rate, 0) > 1");
        } else if columns.contains("utilization_pct") {
            builder.push(" AND COALESCE(utilization_pct, 0) > 100");
        } else if columns.contains("occupied_qty") && columns.contains("capacity") {
            builder.push(" AND COALESCE(occupied_qty, 0) > COALESCE(capacity, 0)");
        } else if columns.contains("current_qty") && columns.contains("capacity") {
            builder.push(" AND COALESCE(current_qty, 0) > COALESCE(capacity, 0)");
        } else if columns.contains("available_capacity") {
            builder.push(" AND COALESCE(available_capacity, 0) < 0");
        }
    }

    if filter.only_occupied {
        if columns.contains("occupied_qty") {
            builder.push(" AND COALESCE(occupied_qty, 0) > 0");
        } else if columns.contains("current_qty") {
            builder.push(" AND COALESCE(current_qty, 0) > 0");
        } else if columns.contains("stock_line_count") {
            builder.push(" AND COALESCE(stock_line_count, 0) > 0");
        } else if columns.contains("material_count") {
            builder.push(" AND COALESCE(material_count, 0) > 0");
        }
    }
}

fn bin_stock_summary_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("zone_code") && columns.contains("bin_code") {
        "zone_code ASC, bin_code ASC"
    } else if columns.contains("zone") && columns.contains("bin_code") {
        "zone ASC, bin_code ASC"
    } else if columns.contains("bin_code") {
        "bin_code ASC"
    } else {
        "1 ASC"
    }
}

fn apply_batch_stock_summary_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a BatchStockSummaryReportFilter,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", filter.material_id.clone());
    push_optional_equals_filter(
        builder,
        columns,
        "batch_number",
        filter.batch_number.clone(),
    );
    push_optional_equals_filter(
        builder,
        columns,
        "quality_status",
        filter.quality_status.clone(),
    );

    if let Some(expiry_date_before) = filter.expiry_date_before {
        if let Some(expiry_column) =
            first_existing_column(columns, &["expiry_date", "expiration_date"])
        {
            builder.push(" AND ");
            builder.push(expiry_column);
            builder.push(" <= ");
            builder.push_bind(expiry_date_before);
        }
    }

    if filter.only_expired {
        if let Some(expiry_column) =
            first_existing_column(columns, &["expiry_date", "expiration_date"])
        {
            builder.push(" AND ");
            builder.push(expiry_column);
            builder.push(" < CURRENT_DATE");
        } else if columns.contains("days_to_expiry") {
            builder.push(" AND COALESCE(days_to_expiry, 0) < 0");
        }
    }

    if filter.only_expiring {
        if columns.contains("days_to_expiry") {
            builder.push(" AND COALESCE(days_to_expiry, 999999) BETWEEN 0 AND 30");
        } else if let Some(expiry_column) =
            first_existing_column(columns, &["expiry_date", "expiration_date"])
        {
            builder.push(" AND ");
            builder.push(expiry_column);
            builder.push(" >= CURRENT_DATE AND ");
            builder.push(expiry_column);
            builder.push(" <= CURRENT_DATE + INTERVAL '30 days'");
        }
    }
}

fn batch_stock_summary_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("fefo_rank") {
        "fefo_rank ASC"
    } else if columns.contains("expiry_date") && columns.contains("material_id") {
        "expiry_date ASC NULLS LAST, material_id ASC, batch_number ASC"
    } else if columns.contains("material_id") && columns.contains("batch_number") {
        "material_id ASC, batch_number ASC"
    } else if columns.contains("batch_number") {
        "batch_number ASC"
    } else {
        "1 ASC"
    }
}

fn apply_data_consistency_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    filter: &'a DataConsistencyReportFilter,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", filter.material_id.clone());

    if filter.only_inconsistent {
        if columns.contains("is_consistent") {
            builder.push(" AND is_consistent IS NOT TRUE");
            return;
        }

        if columns.contains("check_status") {
            builder.push(" AND check_status <> '一致'");
            return;
        }

        let has_material_vs_bin = columns.contains("difference_material_vs_bin")
            || columns.contains("diff_material_vs_bin");
        let has_material_vs_batch = columns.contains("difference_material_vs_batch")
            || columns.contains("diff_material_vs_batch");

        if has_material_vs_bin || has_material_vs_batch {
            builder.push(" AND (");
            let mut pushed = false;

            if has_material_vs_bin {
                if columns.contains("difference_material_vs_bin") {
                    builder.push("COALESCE(difference_material_vs_bin, 0) <> 0");
                } else {
                    builder.push("COALESCE(diff_material_vs_bin, 0) <> 0");
                }
                pushed = true;
            }

            if has_material_vs_batch {
                if pushed {
                    builder.push(" OR ");
                }
                if columns.contains("difference_material_vs_batch") {
                    builder.push("COALESCE(difference_material_vs_batch, 0) <> 0");
                } else {
                    builder.push("COALESCE(diff_material_vs_batch, 0) <> 0");
                }
            }

            builder.push(")");
        }
    }
}

fn data_consistency_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("is_consistent") && columns.contains("material_id") {
        "is_consistent ASC, material_id ASC"
    } else if columns.contains("check_status") && columns.contains("material_id") {
        "CASE WHEN check_status = '一致' THEN 1 ELSE 0 END ASC, material_id ASC"
    } else if columns.contains("material_id") {
        "material_id ASC"
    } else {
        "1 ASC"
    }
}

async fn load_report_columns(
    pool: &sqlx::PgPool,
    schema: &str,
    view_name: &str,
) -> AppResult<HashSet<String>> {
    let columns: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT column_name
        FROM information_schema.columns
        WHERE table_schema = $1
          AND table_name = $2
        "#,
    )
    .bind(schema)
    .bind(view_name)
    .fetch_all(pool)
    .await
    .map_err(|err| match map_reporting_db_error(err) {
        AppError::Business {
            code: "REPORT_QUERY_FAILED",
            ..
        } => AppError::business("REPORT_QUERY_FAILED", "读取报表字段失败"),
        mapped => mapped,
    })?;

    Ok(columns.into_iter().collect())
}

fn json_rows_to_csv(rows: &[Value], include_headers: bool) -> String {
    let mut headers: Vec<String> = Vec::new();

    for row in rows {
        if let Some(object) = row.as_object() {
            for key in object.keys() {
                if !headers.iter().any(|existing| existing == key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    let mut csv = String::new();

    if include_headers {
        csv.push_str(
            &headers
                .iter()
                .map(|header| csv_escape_cell(header))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }

    for row in rows {
        let Some(object) = row.as_object() else {
            continue;
        };

        let line = headers
            .iter()
            .map(|header| {
                object
                    .get(header)
                    .map(json_value_to_csv_cell)
                    .unwrap_or_default()
            })
            .map(|cell| csv_escape_cell(&cell))
            .collect::<Vec<_>>()
            .join(",");

        csv.push_str(&line);
        csv.push('\n');
    }

    csv
}

fn json_value_to_csv_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn csv_escape_cell(value: &str) -> String {
    let needs_quotes =
        value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r');

    if needs_quotes {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn first_existing_column<'a>(columns: &HashSet<String>, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .copied()
        .find(|candidate| columns.contains(*candidate))
}

fn push_optional_equals_filter_unchecked<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    column: &'static str,
    value: Option<String>,
) {
    let Some(value) = normalize_optional_string(value) else {
        return;
    };

    builder.push(" AND ");
    builder.push(column);
    builder.push(" = ");
    builder.push_bind(value);
}

fn push_optional_like_filter_unchecked<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    column: &'static str,
    value: Option<String>,
) {
    let Some(value) = normalize_optional_string(value) else {
        return;
    };

    builder.push(" AND ");
    builder.push(column);
    builder.push(" ILIKE ");
    builder.push_bind(format!("%{value}%"));
}

fn push_optional_equals_filter<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    columns: &HashSet<String>,
    column: &'static str,
    value: Option<String>,
) {
    if !columns.contains(column) {
        return;
    }

    push_optional_equals_filter_unchecked(builder, column, value);
}

fn push_optional_equals_filter_by_candidates<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    columns: &HashSet<String>,
    candidates: &[&'static str],
    value: Option<String>,
) {
    let Some(column) = first_existing_column(columns, candidates) else {
        return;
    };

    push_optional_equals_filter_unchecked(builder, column, value);
}

fn report_slug(report_type: ReportType) -> &'static str {
    match report_type {
        ReportType::CurrentStock => "current-stock",
        ReportType::InventoryValue => "inventory-value",
        ReportType::QualityStatus => "quality-status",
        ReportType::MrpShortage => "mrp-shortage",
        ReportType::LowStockAlert => "low-stock-alert",
        ReportType::StockByZone => "stock-by-zone",
        ReportType::BinStockSummary => "bin-stock-summary",
        ReportType::BatchStockSummary => "batch-stock-summary",
        ReportType::DataConsistency => "data-consistency",
    }
}

fn report_display_name(report_type: ReportType) -> &'static str {
    match report_type {
        ReportType::CurrentStock => "当前库存报表",
        ReportType::InventoryValue => "库存价值报表",
        ReportType::QualityStatus => "质量状态报表",
        ReportType::MrpShortage => "MRP 短缺报表",
        ReportType::LowStockAlert => "低库存预警报表",
        ReportType::StockByZone => "区域库存矩阵",
        ReportType::BinStockSummary => "货位库存汇总",
        ReportType::BatchStockSummary => "批次库存汇总",
        ReportType::DataConsistency => "数据一致性检查",
    }
}
