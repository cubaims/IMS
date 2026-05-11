use super::dto::{
    CancelMrpSuggestionRequest, CancelMrpSuggestionResponse, ConfirmMrpSuggestionRequest,
    ConfirmMrpSuggestionResponse, MrpResponse, MrpRunsQueryRequest,
    MrpSuggestionsExportQueryRequest, MrpSuggestionsQueryRequest, RunMrpRequest, RunMrpResponse,
};
use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState, CurrentUser, write_audit_event};
use rust_decimal::Decimal;
use time::Time;

use crate::{
    application::{
        CancelMrpSuggestionCommand, CancelMrpSuggestionUseCase, ConfirmMrpSuggestionCommand,
        ConfirmMrpSuggestionUseCase, GetMrpRunUseCase, GetMrpSuggestionUseCase, ListMrpRunsUseCase,
        ListMrpSuggestionsUseCase, MrpRunQuery, MrpSuggestionQuery, MrpSuggestionRepository,
        RunMrpCommand, RunMrpUseCase,
    },
    domain::{
        MaterialId, MrpRunId, MrpSuggestionId, MrpSuggestionStatus, MrpSuggestionType, Operator,
        ProductVariantId,
    },
    infrastructure::{PostgresMrpIdGenerator, PostgresMrpStore},
};

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

fn suggestion_type_code(value: MrpSuggestionType) -> &'static str {
    match value {
        MrpSuggestionType::Purchase => "PURCHASE",
        MrpSuggestionType::Production => "PRODUCTION",
        MrpSuggestionType::Transfer => "TRANSFER",
    }
}

fn suggestion_status_code(value: MrpSuggestionStatus) -> &'static str {
    match value {
        MrpSuggestionStatus::Open => "OPEN",
        MrpSuggestionStatus::Confirmed => "CONFIRMED",
        MrpSuggestionStatus::Cancelled => "CANCELLED",
        MrpSuggestionStatus::Converted => "CONVERTED",
    }
}

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MrpResponse>>> {
    Ok(Json(ApiResponse::ok(MrpResponse {
        module: "mrp",
        status: "ready",
    })))
}

pub async fn run(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<RunMrpRequest>,
) -> AppResult<Json<ApiResponse<RunMrpResponse>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let id_generator = PostgresMrpIdGenerator::default();

    let use_case = RunMrpUseCase::new(store.clone(), store.clone(), store.clone(), id_generator);

    if request.demand_qty <= Decimal::ZERO {
        return Err(AppError::Business {
            code: "MRP_DEMAND_INVALID",
            message: "需求数量必须大于 0".to_string(),
        });
    }

    let variant_code = normalize_optional_string(request.variant_code);
    let finished_material_id = normalize_optional_string(request.finished_material_id);
    let response_finished_material_id = finished_material_id.clone();

    let command = RunMrpCommand {
        material_id: finished_material_id.map(MaterialId::new),
        product_variant_id: variant_code.map(ProductVariantId::new),
        demand_qty: request.demand_qty,
        demand_date: request.demand_date.with_time(Time::MIDNIGHT).assume_utc(),
        created_by: Operator::new(user.username.clone()),
        remark: normalize_optional_string(request.remark),
    };

    let output = use_case.execute(command).await?;
    let (suggestion_count, shortage_count) =
        store.count_suggestions_for_run(&output.run_id).await?;

    let response = RunMrpResponse {
        run_id: output.run_id,
        status: output.status,
        variant_code: output
            .product_variant_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        finished_material_id: response_finished_material_id,
        demand_qty: request.demand_qty,
        demand_date: request.demand_date,
        suggestion_count,
        shortage_count,
    };

    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "MRP_RUN",
        Some("wms.wms_mrp_runs"),
        Some(response.run_id.as_str()),
        Some(serde_json::json!({
            "run_id": response.run_id.as_str(),
            "status": response.status,
            "variant_code": response.variant_code.as_deref(),
            "finished_material_id": response.finished_material_id.as_deref(),
            "demand_qty": response.demand_qty,
            "demand_date": response.demand_date,
            "suggestion_count": response.suggestion_count,
            "shortage_count": response.shortage_count
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(response)))
}

pub async fn runs(
    State(state): State<AppState>,
    Query(request): Query<MrpRunsQueryRequest>,
) -> AppResult<Json<ApiResponse<cuba_shared::Page<crate::application::MrpRunSummary>>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());

    if let (Some(date_from), Some(date_to)) = (request.date_from, request.date_to) {
        if date_from >= date_to {
            return Err(AppError::Business {
                code: "MRP_QUERY_INVALID",
                message: "date_from 必须早于 date_to".to_string(),
            });
        }
    }

    let variant_code = normalize_optional_string(request.variant_code);
    let finished_material_id = normalize_optional_string(request.finished_material_id);

    let query = MrpRunQuery {
        page: cuba_shared::PageQuery {
            page: request.page.unwrap_or(1),
            page_size: request.page_size.unwrap_or(20),
        },
        status: request.status,
        material_id: finished_material_id.map(MaterialId::new),
        product_variant_id: variant_code.map(ProductVariantId::new),
        date_from: request.date_from,
        date_to: request.date_to,
    };

    let use_case = ListMrpRunsUseCase::new(store);
    let result = use_case.execute(query).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::MrpRun>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let run_id = MrpRunId::new(run_id);

    let use_case = GetMrpRunUseCase::new(store);
    let run = use_case.execute(run_id).await?;

    Ok(Json(ApiResponse::ok(run)))
}

pub async fn suggestions(
    State(state): State<AppState>,
    Query(request): Query<MrpSuggestionsQueryRequest>,
) -> AppResult<Json<ApiResponse<cuba_shared::Page<crate::domain::MrpSuggestion>>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());

    if let (Some(date_from), Some(date_to)) = (request.date_from, request.date_to) {
        if date_from >= date_to {
            return Err(AppError::Business {
                code: "MRP_QUERY_INVALID",
                message: "date_from 必须早于 date_to".to_string(),
            });
        }
    }

    let run_id = normalize_optional_string(request.run_id);
    let material_id = normalize_optional_string(request.material_id);

    let query = MrpSuggestionQuery {
        page: cuba_shared::PageQuery {
            page: request.page.unwrap_or(1),
            page_size: request.page_size.unwrap_or(20),
        },
        run_id: run_id.map(MrpRunId::new),
        suggestion_type: request.suggestion_type,
        status: request.status,
        material_id: material_id.map(MaterialId::new),
        required_date_from: request.date_from,
        required_date_to: request.date_to,
        only_shortage: request.only_shortage,
    };

    let use_case = ListMrpSuggestionsUseCase::new(store);
    let result = use_case.execute(query).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn suggestions_export(
    State(state): State<AppState>,
    Query(request): Query<MrpSuggestionsExportQueryRequest>,
) -> AppResult<Response> {
    let format =
        normalize_optional_string(request.format.clone()).unwrap_or_else(|| "csv".to_string());

    if !format.eq_ignore_ascii_case("csv") {
        return Err(AppError::business(
            "REPORT_FORMAT_UNSUPPORTED",
            "MRP 建议导出 MVP 仅支持 csv",
        ));
    }

    if let (Some(date_from), Some(date_to)) = (request.date_from, request.date_to) {
        if date_from >= date_to {
            return Err(AppError::Business {
                code: "MRP_QUERY_INVALID",
                message: "date_from 必须早于 date_to".to_string(),
            });
        }
    }

    let store = PostgresMrpStore::new(state.db_pool.clone());
    let run_id = normalize_optional_string(request.run_id);
    let material_id = normalize_optional_string(request.material_id);

    let mut query = MrpSuggestionQuery {
        page: cuba_shared::PageQuery {
            page: 1,
            page_size: 200,
        },
        run_id: run_id.map(MrpRunId::new),
        suggestion_type: request.suggestion_type,
        status: request.status,
        material_id: material_id.map(MaterialId::new),
        required_date_from: request.date_from,
        required_date_to: request.date_to,
        only_shortage: request.only_shortage,
    };

    let mut all_items = Vec::new();

    loop {
        let result =
            <PostgresMrpStore as MrpSuggestionRepository>::list(&store, query.clone()).await?;
        let total = result.total;
        let item_count = result.items.len();

        all_items.extend(result.items);

        if all_items.len() as u64 >= total || item_count == 0 {
            break;
        }

        query.page.page = query.page.page.saturating_add(1);
    }

    let include_headers = request.include_headers.unwrap_or(true);

    let mut csv = String::new();

    if include_headers {
        csv.push_str(
            &[
                "suggestion_id",
                "run_id",
                "material_id",
                "suggestion_type",
                "required_qty",
                "available_qty",
                "net_requirement_qty",
                "shortage_qty",
                "suggested_qty",
                "suggested_date",
                "status",
                "priority",
                "remark",
            ]
            .join(","),
        );
        csv.push('\n');
    }

    for item in all_items {
        let line = [
            item.id.as_str().to_string(),
            item.run_id.as_str().to_string(),
            item.material_id.as_str().to_string(),
            suggestion_type_code(item.suggestion_type).to_string(),
            item.required_qty.to_string(),
            item.available_qty.to_string(),
            item.net_requirement_qty.to_string(),
            item.shortage_qty.to_string(),
            item.suggested_qty.to_string(),
            item.suggested_date.to_string(),
            suggestion_status_code(item.status).to_string(),
            item.priority
                .map(|value| value.to_string())
                .unwrap_or_default(),
            item.remark.unwrap_or_default(),
        ]
        .into_iter()
        .map(|cell| csv_escape_cell(&cell))
        .collect::<Vec<_>>()
        .join(",");

        csv.push_str(&line);
        csv.push('\n');
    }

    csv_response("mrp-suggestions.csv", csv)
}

pub async fn confirm_suggestion(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(suggestion_id): Path<String>,
    Json(request): Json<ConfirmMrpSuggestionRequest>,
) -> AppResult<Json<ApiResponse<ConfirmMrpSuggestionResponse>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let use_case = ConfirmMrpSuggestionUseCase::new(store);
    let suggestion_id = MrpSuggestionId::new(suggestion_id);

    let output = use_case
        .execute(ConfirmMrpSuggestionCommand {
            suggestion_id,
            confirmed_by: Operator::new(user.username.clone()),
            remark: request.remark,
        })
        .await?;

    let response = ConfirmMrpSuggestionResponse {
        suggestion_id: output.suggestion_id,
        status: output.status,
    };

    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "MRP_SUGGESTION_CONFIRM",
        Some("wms.wms_mrp_suggestions"),
        Some(response.suggestion_id.as_str()),
        Some(serde_json::json!({
            "suggestion_id": response.suggestion_id.as_str(),
            "status": response.status
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(response)))
}

pub async fn cancel_suggestion(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(suggestion_id): Path<String>,
    Json(request): Json<CancelMrpSuggestionRequest>,
) -> AppResult<Json<ApiResponse<CancelMrpSuggestionResponse>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let use_case = CancelMrpSuggestionUseCase::new(store);
    let reason = request.reason;
    let suggestion_id = MrpSuggestionId::new(suggestion_id);

    let output = use_case
        .execute(CancelMrpSuggestionCommand {
            suggestion_id,
            cancelled_by: Operator::new(user.username.clone()),
            reason: reason.clone(),
        })
        .await?;

    let response = CancelMrpSuggestionResponse {
        suggestion_id: output.suggestion_id,
        status: output.status,
    };

    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "MRP_SUGGESTION_CANCEL",
        Some("wms.wms_mrp_suggestions"),
        Some(response.suggestion_id.as_str()),
        Some(serde_json::json!({
            "suggestion_id": response.suggestion_id.as_str(),
            "status": response.status,
            "reason": reason
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(response)))
}

pub async fn get_suggestion(
    State(state): State<AppState>,
    Path(suggestion_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::MrpSuggestion>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let suggestion_id = MrpSuggestionId::new(suggestion_id);

    let use_case = GetMrpSuggestionUseCase::new(store);
    let suggestion = use_case.execute(suggestion_id).await?;

    Ok(Json(ApiResponse::ok(suggestion)))
}
