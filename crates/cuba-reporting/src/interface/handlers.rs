use super::dto::{
    BatchStockSummaryExportQuery, BatchStockSummaryReportQuery, BinStockSummaryReportQuery,
    CurrentStockExportQuery, CurrentStockReportQuery, DataConsistencyExportQuery,
    DataConsistencyReportQuery, InventoryValueExportQuery, InventoryValueReportQuery,
    LowStockAlertExportQuery, LowStockAlertReportQuery, MrpShortageExportQuery,
    MrpShortageReportQuery, QualityStatusReportQuery, ReportingResponse, StockByZoneReportQuery,
};
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState, Page};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::HashSet;

fn apply_current_stock_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a CurrentStockReportQuery,
) {
    let material_id = normalize_optional_string(query.material_id.clone());
    let material_name = normalize_optional_string(query.material_name.clone());
    let bin_code = normalize_optional_string(query.bin_code.clone());
    let batch_number = normalize_optional_string(query.batch_number.clone());
    let quality_status = normalize_optional_string(query.quality_status.clone());
    let zone_code = normalize_optional_string(query.zone_code.clone());

    if let Some(value) = material_id {
        builder.push(" AND material_id = ");
        builder.push_bind(value);
    }

    if let Some(value) = material_name {
        builder.push(" AND material_name ILIKE ");
        builder.push_bind(format!("%{value}%"));
    }

    if let Some(value) = bin_code {
        builder.push(" AND bin_code = ");
        builder.push_bind(value);
    }

    if let Some(value) = batch_number {
        builder.push(" AND batch_number = ");
        builder.push_bind(value);
    }

    if let Some(value) = quality_status {
        builder.push(" AND quality_status = ");
        builder.push_bind(value);
    }

    if let Some(value) = zone_code {
        builder.push(" AND zone_code = ");
        builder.push_bind(value);
    }

    if query.only_available.unwrap_or(false) {
        // 视图里若字段名为 available_qty，则按可用量过滤。
        // 如果你的 v9 视图实际字段不是 available_qty，
        // 下一批根据数据库字段名改这里即可。
        builder.push(" AND COALESCE(available_qty, 0) > 0");
    }
}

fn apply_inventory_value_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a InventoryValueReportQuery,
) {
    let material_id = normalize_optional_string(query.material_id.clone());
    let material_type = normalize_optional_string(query.material_type.clone());

    if let Some(value) = material_id {
        builder.push(" AND material_id = ");
        builder.push_bind(value);
    }

    if let Some(value) = material_type {
        builder.push(" AND material_type = ");
        builder.push_bind(value);
    }

    if query.only_positive_value.unwrap_or(false) {
        builder.push(" AND COALESCE(inventory_value, 0) > 0");
    }
}

fn inventory_value_order_by(query: &InventoryValueReportQuery) -> &'static str {
    let sort_by = normalize_optional_string(query.sort_by.clone())
        .unwrap_or_else(|| "inventory_value".to_string());

    let desc = normalize_optional_string(query.sort_order.clone())
        .map(|v| v.eq_ignore_ascii_case("desc"))
        .unwrap_or(true);

    match (sort_by.as_str(), desc) {
        ("material_id", false) => "material_id ASC",
        ("material_id", true) => "material_id DESC",
        ("material_type", false) => "material_type ASC, material_id ASC",
        ("material_type", true) => "material_type DESC, material_id ASC",
        ("current_stock", false) => "current_stock ASC, material_id ASC",
        ("current_stock", true) => "current_stock DESC, material_id ASC",
        ("standard_cost", false) => "standard_cost ASC, material_id ASC",
        ("standard_cost", true) => "standard_cost DESC, material_id ASC",
        ("map_price", false) => "map_price ASC, material_id ASC",
        ("map_price", true) => "map_price DESC, material_id ASC",
        ("price_variance", false) => "price_variance ASC, material_id ASC",
        ("price_variance", true) => "price_variance DESC, material_id ASC",
        ("inventory_value", false) | ("value", false) => "inventory_value ASC, material_id ASC",
        _ => "inventory_value DESC, material_id ASC",
    }
}

fn apply_quality_status_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a QualityStatusReportQuery,
) {
    let material_id = normalize_optional_string(query.material_id.clone());
    let quality_status = normalize_optional_string(query.quality_status.clone());
    let batch_number = normalize_optional_string(query.batch_number.clone());

    if let Some(value) = material_id {
        builder.push(" AND material_id = ");
        builder.push_bind(value);
    }

    if let Some(value) = quality_status {
        builder.push(" AND quality_status = ");
        builder.push_bind(value);
    }

    if let Some(value) = batch_number {
        builder.push(" AND batch_number = ");
        builder.push_bind(value);
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
        .map_err(|err| AppError::business("REPORT_QUERY_FAILED", format!("读取报表字段失败: {err}")))?;

    Ok(columns.into_iter().collect())
}

fn apply_mrp_shortage_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a MrpShortageReportQuery,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "run_id", query.run_id.clone());
    push_optional_equals_filter(builder, columns, "material_id", query.material_id.clone());

    if let Some(value) = normalize_optional_string(query.suggestion_type.clone()) {
        if let Some(column) =
            first_existing_column(columns, &["suggestion_type", "suggested_order_type"])
        {
            builder.push(" AND ");
            builder.push(column);
            builder.push(" = ");
            builder.push_bind(value);
        }
    }

    if query.only_open.unwrap_or(false) {
        if columns.contains("status") {
            builder.push(" AND status IN ('OPEN', 'NEW', '待处理', '新建')");
        }
    }

    if let Some(date_column) = first_existing_column(
        columns,
        &[
            "required_date",
            "suggested_date",
            "demand_date",
            "created_at",
        ],
    ) {
        if let Some(date_from) = query.date_from {
            builder.push(" AND ");
            builder.push(date_column);
            builder.push(" >= ");
            builder.push_bind(date_from);
        }

        if let Some(date_to) = query.date_to {
            builder.push(" AND ");
            builder.push(date_column);
            builder.push(" < ");
            builder.push_bind(date_to);
        }
    }
}

fn apply_low_stock_alert_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a LowStockAlertReportQuery,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", query.material_id.clone());
    push_optional_equals_filter(
        builder,
        columns,
        "material_type",
        query.material_type.clone(),
    );
    push_optional_equals_filter(builder, columns, "severity", query.severity.clone());
}

fn low_stock_alert_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("severity") && columns.contains("material_id") {
        "severity DESC, material_id ASC"
    } else if columns.contains("material_id") {
        "material_id ASC"
    } else {
        "1 ASC"
    }
}

fn apply_data_consistency_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a DataConsistencyReportQuery,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", query.material_id.clone());

    if query.only_inconsistent.unwrap_or(false) {
        if columns.contains("is_consistent") {
            builder.push(" AND is_consistent IS NOT TRUE");
            return;
        }

        let has_material_vs_bin = columns.contains("difference_material_vs_bin");
        let has_material_vs_batch = columns.contains("difference_material_vs_batch");

        if has_material_vs_bin || has_material_vs_batch {
            builder.push(" AND (");

            let mut pushed = false;

            if has_material_vs_bin {
                builder.push("COALESCE(difference_material_vs_bin, 0) <> 0");
                pushed = true;
            }

            if has_material_vs_batch {
                if pushed {
                    builder.push(" OR ");
                }
                builder.push("COALESCE(difference_material_vs_batch, 0) <> 0");
            }

            builder.push(")");
        }
    }
}

fn data_consistency_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("is_consistent") && columns.contains("material_id") {
        "is_consistent ASC, material_id ASC"
    } else if columns.contains("material_id") {
        "material_id ASC"
    } else {
        "1 ASC"
    }
}

fn apply_stock_by_zone_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a StockByZoneReportQuery,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", query.material_id.clone());
    push_optional_equals_filter(
        builder,
        columns,
        "material_type",
        query.material_type.clone(),
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
    query: &'a BinStockSummaryReportQuery,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "bin_code", query.bin_code.clone());
    push_optional_equals_filter(builder, columns, "zone_code", query.zone_code.clone());

    if query.only_over_capacity.unwrap_or(false) {
        if columns.contains("capacity_usage_rate") {
            builder.push(" AND COALESCE(capacity_usage_rate, 0) > 1");
        } else if columns.contains("occupied_qty") && columns.contains("capacity") {
            builder.push(" AND COALESCE(occupied_qty, 0) > COALESCE(capacity, 0)");
        } else if columns.contains("available_capacity") {
            builder.push(" AND COALESCE(available_capacity, 0) < 0");
        }
    }

    if query.only_occupied.unwrap_or(false) {
        if columns.contains("occupied_qty") {
            builder.push(" AND COALESCE(occupied_qty, 0) > 0");
        } else if columns.contains("stock_line_count") {
            builder.push(" AND COALESCE(stock_line_count, 0) > 0");
        }
    }
}

fn bin_stock_summary_order_by(columns: &HashSet<String>) -> &'static str {
    if columns.contains("zone_code") && columns.contains("bin_code") {
        "zone_code ASC, bin_code ASC"
    } else if columns.contains("bin_code") {
        "bin_code ASC"
    } else {
        "1 ASC"
    }
}

fn apply_batch_stock_summary_filters<'a>(
    builder: &mut QueryBuilder<'a, Postgres>,
    query: &'a BatchStockSummaryReportQuery,
    columns: &HashSet<String>,
) {
    push_optional_equals_filter(builder, columns, "material_id", query.material_id.clone());
    push_optional_equals_filter(builder, columns, "batch_number", query.batch_number.clone());
    push_optional_equals_filter(
        builder,
        columns,
        "quality_status",
        query.quality_status.clone(),
    );

    if let Some(expiry_date_before) = query.expiry_date_before {
        if let Some(expiry_column) =
            first_existing_column(columns, &["expiry_date", "expiration_date"])
        {
            builder.push(" AND ");
            builder.push(expiry_column);
            builder.push(" <= ");
            builder.push_bind(expiry_date_before);
        }
    }

    if query.only_expired.unwrap_or(false) {
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

    if query.only_expiring.unwrap_or(false) {
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

fn csv_escape_cell(value: &str) -> String {
    let needs_quotes =
        value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r');

    if needs_quotes {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
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

fn csv_response(filename: &str, body: String) -> AppResult<Response> {
    let content_type = HeaderValue::from_static("text/csv; charset=utf-8");
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(|err| {
            AppError::business("REPORT_EXPORT_FAILED", format!("生成导出响应头失败: {err}"))
        })?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        body,
    )
        .into_response())
}

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
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_current_stock WHERE 1 = 1");
    apply_current_stock_filters(&mut count_builder, &query);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("当前库存报表统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_current_stock WHERE 1 = 1",
    );
    apply_current_stock_filters(&mut data_builder, &query);
    data_builder.push(" ORDER BY material_id, bin_code, batch_number LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("当前库存报表查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("当前库存报表结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn current_stock_export(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockExportQuery>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(query.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "当前库存报表导出 MVP 仅支持 csv",
        ));
    }

    let current_stock_query = CurrentStockReportQuery {
        material_id: query.material_id,
        material_name: query.material_name,
        bin_code: query.bin_code,
        batch_number: query.batch_number,
        quality_status: query.quality_status,
        zone_code: query.zone_code,
        only_available: query.only_available,
        page: None,
        page_size: None,
    };

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_current_stock WHERE 1 = 1",
    );
    apply_current_stock_filters(&mut data_builder, &current_stock_query);
    data_builder.push(" ORDER BY material_id, bin_code, batch_number");
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("当前库存报表导出查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("当前库存报表导出结果转换失败: {err}"),
            )
        })?;

    let include_headers = query.include_headers.unwrap_or(true);
    let csv = json_rows_to_csv(&items, include_headers);

    csv_response("current-stock.csv", csv)
}

pub async fn inventory_value(
    State(state): State<AppState>,
    Query(query): Query<InventoryValueReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_inventory_value WHERE 1 = 1");
    apply_inventory_value_filters(&mut count_builder, &query);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("库存价值报表统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_inventory_value WHERE 1 = 1",
    );
    apply_inventory_value_filters(&mut data_builder, &query);
    data_builder.push(" ORDER BY ");
    data_builder.push(inventory_value_order_by(&query));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("库存价值报表查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("库存价值报表结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn inventory_value_export(
    State(state): State<AppState>,
    Query(query): Query<InventoryValueExportQuery>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(query.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "库存价值报表导出 MVP 仅支持 csv",
        ));
    }

    let inventory_value_query = InventoryValueReportQuery {
        material_id: query.material_id,
        material_type: query.material_type,
        only_positive_value: query.only_positive_value,
        sort_by: query.sort_by,
        sort_order: query.sort_order,
        page: None,
        page_size: None,
    };

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_inventory_value WHERE 1 = 1",
    );
    apply_inventory_value_filters(&mut data_builder, &inventory_value_query);
    data_builder.push(" ORDER BY ");
    data_builder.push(inventory_value_order_by(&inventory_value_query));
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("库存价值报表导出查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("库存价值报表导出结果转换失败: {err}"),
            )
        })?;

    let include_headers = query.include_headers.unwrap_or(true);
    let csv = json_rows_to_csv(&items, include_headers);

    csv_response("inventory-value.csv", csv)
}

pub async fn quality_status(
    State(state): State<AppState>,
    Query(query): Query<QualityStatusReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_quality_status WHERE 1 = 1");
    apply_quality_status_filters(&mut count_builder, &query);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("质量状态报表统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_quality_status WHERE 1 = 1",
    );
    apply_quality_status_filters(&mut data_builder, &query);
    data_builder.push(" ORDER BY material_id, quality_status, batch_number LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("质量状态报表查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("质量状态报表结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn mrp_shortage(
    State(state): State<AppState>,
    Query(query): Query<MrpShortageReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    if let (Some(date_from), Some(date_to)) = (query.date_from, query.date_to) {
        if date_from >= date_to {
            return Err(AppError::business(
                "REPORT_QUERY_INVALID",
                "date_from 必须早于 date_to",
            ));
        }
    }

    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_mrp_shortage").await?;
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_mrp_shortage WHERE 1 = 1");
    apply_mrp_shortage_filters(&mut count_builder, &query, &columns);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("MRP 短缺报表统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_mrp_shortage WHERE 1 = 1",
    );
    apply_mrp_shortage_filters(&mut data_builder, &query, &columns);
    data_builder.push(" ORDER BY ");

    if let Some(column) = first_existing_column(&columns, &["priority", "material_id", "run_id"]) {
        data_builder.push(column);
        data_builder.push(" ASC");
    } else {
        data_builder.push("1 ASC");
    }

    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("MRP 短缺报表查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("MRP 短缺报表结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn mrp_shortage_export(
    State(state): State<AppState>,
    Query(query): Query<MrpShortageExportQuery>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(query.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "MRP 短缺报表导出 MVP 仅支持 csv",
        ));
    }

    if let (Some(date_from), Some(date_to)) = (query.date_from, query.date_to) {
        if date_from >= date_to {
            return Err(AppError::business(
                "REPORT_QUERY_INVALID",
                "date_from 必须早于 date_to",
            ));
        }
    }

    let report_query = MrpShortageReportQuery {
        run_id: query.run_id,
        material_id: query.material_id,
        suggestion_type: query.suggestion_type,
        only_open: query.only_open,
        date_from: query.date_from,
        date_to: query.date_to,
        page: None,
        page_size: None,
    };

    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_mrp_shortage").await?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_mrp_shortage WHERE 1 = 1",
    );
    apply_mrp_shortage_filters(&mut data_builder, &report_query, &columns);
    data_builder.push(" ORDER BY ");

    if let Some(column) = first_existing_column(&columns, &["priority", "material_id", "run_id"]) {
        data_builder.push(column);
        data_builder.push(" ASC");
    } else {
        data_builder.push("1 ASC");
    }

    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("MRP 短缺报表导出查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("MRP 短缺报表导出结果转换失败: {err}"),
            )
        })?;

    let include_headers = query.include_headers.unwrap_or(true);
    let csv = json_rows_to_csv(&items, include_headers);

    csv_response("mrp-shortage.csv", csv)
}

pub async fn low_stock_alert(
    State(state): State<AppState>,
    Query(query): Query<LowStockAlertReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_low_stock_alert").await?;
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_low_stock_alert WHERE 1 = 1");
    apply_low_stock_alert_filters(&mut count_builder, &query, &columns);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("低库存预警报表统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_low_stock_alert WHERE 1 = 1",
    );
    apply_low_stock_alert_filters(&mut data_builder, &query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(low_stock_alert_order_by(&columns));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("低库存预警报表查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("低库存预警报表结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn batch_stock_summary_export(
    State(state): State<AppState>,
    Query(query): Query<BatchStockSummaryExportQuery>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(query.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "批次库存汇总报表导出 MVP 仅支持 csv",
        ));
    }

    let report_query = BatchStockSummaryReportQuery {
        material_id: query.material_id,
        batch_number: query.batch_number,
        quality_status: query.quality_status,
        only_expiring: query.only_expiring,
        only_expired: query.only_expired,
        expiry_date_before: query.expiry_date_before,
        page: None,
        page_size: None,
    };

    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_batch_stock_summary").await?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_batch_stock_summary WHERE 1 = 1",
    );
    apply_batch_stock_summary_filters(&mut data_builder, &report_query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(batch_stock_summary_order_by(&columns));
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("批次库存汇总报表导出查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("批次库存汇总报表导出结果转换失败: {err}"),
            )
        })?;

    let include_headers = query.include_headers.unwrap_or(true);
    let csv = json_rows_to_csv(&items, include_headers);

    csv_response("batch-stock-summary.csv", csv)
}

pub async fn data_consistency(
    State(state): State<AppState>,
    Query(query): Query<DataConsistencyReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_data_consistency_check").await?;
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM rpt.rpt_data_consistency_check WHERE 1 = 1",
    );
    apply_data_consistency_filters(&mut count_builder, &query, &columns);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("数据一致性检查统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_data_consistency_check WHERE 1 = 1",
    );
    apply_data_consistency_filters(&mut data_builder, &query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(data_consistency_order_by(&columns));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("数据一致性检查查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("数据一致性检查结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn low_stock_alert_export(
    State(state): State<AppState>,
    Query(query): Query<LowStockAlertExportQuery>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(query.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "低库存预警报表导出 MVP 仅支持 csv",
        ));
    }

    let report_query = LowStockAlertReportQuery {
        material_id: query.material_id,
        material_type: query.material_type,
        severity: query.severity,
        page: None,
        page_size: None,
    };

    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_low_stock_alert").await?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_low_stock_alert WHERE 1 = 1",
    );
    apply_low_stock_alert_filters(&mut data_builder, &report_query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(low_stock_alert_order_by(&columns));
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("低库存预警报表导出查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("低库存预警报表导出结果转换失败: {err}"),
            )
        })?;

    let include_headers = query.include_headers.unwrap_or(true);
    let csv = json_rows_to_csv(&items, include_headers);

    csv_response("low-stock-alert.csv", csv)
}

pub async fn stock_by_zone(
    State(state): State<AppState>,
    Query(query): Query<StockByZoneReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_stock_by_zone").await?;
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_stock_by_zone WHERE 1 = 1");
    apply_stock_by_zone_filters(&mut count_builder, &query, &columns);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("区域库存矩阵统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_stock_by_zone WHERE 1 = 1",
    );
    apply_stock_by_zone_filters(&mut data_builder, &query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(stock_by_zone_order_by(&columns));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("区域库存矩阵查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("区域库存矩阵结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn bin_stock_summary(
    State(state): State<AppState>,
    Query(query): Query<BinStockSummaryReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_bin_stock_summary").await?;
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM rpt.rpt_bin_stock_summary WHERE 1 = 1");
    apply_bin_stock_summary_filters(&mut count_builder, &query, &columns);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("货位库存汇总统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_bin_stock_summary WHERE 1 = 1",
    );
    apply_bin_stock_summary_filters(&mut data_builder, &query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(bin_stock_summary_order_by(&columns));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("货位库存汇总查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("货位库存汇总结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn batch_stock_summary(
    State(state): State<AppState>,
    Query(query): Query<BatchStockSummaryReportQuery>,
) -> AppResult<Json<ApiResponse<Page<Value>>>> {
    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_batch_stock_summary").await?;
    let (page, page_size, limit, offset) = normalize_page(query.page, query.page_size);

    let mut count_builder = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM rpt.rpt_batch_stock_summary WHERE 1 = 1",
    );
    apply_batch_stock_summary_filters(&mut count_builder, &query, &columns);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("批次库存汇总统计失败: {err}"),
            )
        })?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_batch_stock_summary WHERE 1 = 1",
    );
    apply_batch_stock_summary_filters(&mut data_builder, &query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(batch_stock_summary_order_by(&columns));
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("批次库存汇总查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_QUERY_FAILED",
                format!("批次库存汇总结果转换失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(Page::new(
        items,
        total.max(0) as u64,
        page,
        page_size,
    ))))
}

pub async fn data_consistency_export(
    State(state): State<AppState>,
    Query(query): Query<DataConsistencyExportQuery>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(query.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "数据一致性检查导出 MVP 仅支持 csv",
        ));
    }

    let report_query = DataConsistencyReportQuery {
        material_id: query.material_id,
        only_inconsistent: query.only_inconsistent,
        page: None,
        page_size: None,
    };

    let columns = load_report_columns(&state.db_pool, "rpt", "rpt_data_consistency_check").await?;

    let mut data_builder = QueryBuilder::<Postgres>::new(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM rpt.rpt_data_consistency_check WHERE 1 = 1",
    );
    apply_data_consistency_filters(&mut data_builder, &report_query, &columns);
    data_builder.push(" ORDER BY ");
    data_builder.push(data_consistency_order_by(&columns));
    data_builder.push(") t");

    let rows = data_builder
        .build()
        .fetch_all(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("数据一致性检查导出查询失败: {err}"),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|row| row.try_get::<Value, _>("row"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            AppError::business(
                "REPORT_EXPORT_FAILED",
                format!("数据一致性检查导出结果转换失败: {err}"),
            )
        })?;

    let include_headers = query.include_headers.unwrap_or(true);
    let csv = json_rows_to_csv(&items, include_headers);

    csv_response("data-consistency.csv", csv)
}

pub async fn refresh(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    sqlx::query("SELECT rpt.refresh_all_materialized_views()")
        .execute(&state.db_pool)
        .await
        .map_err(|err| {
            AppError::business(
                "REPORT_REFRESH_FAILED",
                format!("刷新报表物化视图失败: {err}"),
            )
        })?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "refreshed": true,
        "views": [
            "rpt_current_stock",
            "rpt_inventory_value",
            "rpt_quality_status",
            "rpt_mrp_shortage",
            "rpt_low_stock_alert",
            "rpt_stock_by_zone",
            "rpt_bin_stock_summary",
            "rpt_batch_stock_summary"
        ]
    }))))
}

// ====================== 唯一 Helper 函数（保留这一份，其他全部删除） ======================

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_page(page: Option<u64>, page_size: Option<u64>) -> (u64, u64, i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 200);
    let offset = ((page - 1).saturating_mul(page_size)) as i64;
    let limit = page_size as i64;
    (page, page_size, limit, offset)
}

fn first_existing_column<'a>(columns: &HashSet<String>, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .copied()
        .find(|candidate| columns.contains(*candidate))
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

    let Some(value) = normalize_optional_string(value) else {
        return;
    };

    builder.push(" AND ");
    builder.push(column);
    builder.push(" = ");
    builder.push_bind(value);
}
