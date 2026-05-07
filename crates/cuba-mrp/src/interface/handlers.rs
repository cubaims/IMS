use super::dto::{CancelMrpSuggestionRequest, CancelMrpSuggestionResponse, ConfirmMrpSuggestionRequest, ConfirmMrpSuggestionResponse, MrpResponse, MrpRunsQueryRequest, MrpSuggestionsQueryRequest, RunMrpRequest, RunMrpResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use cuba_shared::{ApiResponse, AppError, AppResult, AppState};
use rust_decimal::Decimal;
use time::Time;

use crate::{
    application::{CancelMrpSuggestionCommand, CancelMrpSuggestionUseCase, ConfirmMrpSuggestionCommand, ConfirmMrpSuggestionUseCase, MrpRunQuery, MrpRunRepository, MrpSuggestionQuery, MrpSuggestionRepository, RunMrpCommand, RunMrpUseCase},
    domain::{MaterialId, MrpRunId, MrpSuggestionId, Operator, ProductVariantId},
    infrastructure::{PostgresMrpIdGenerator, PostgresMrpStore},
};

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn operator_from_headers(headers: &axum::http::HeaderMap) -> Operator {
    let username = headers
        .get("x-user-name")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("API")
        .to_string();

    Operator::new(username)
}

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MrpResponse>>> {
    Ok(Json(ApiResponse::ok(MrpResponse {
        module: "mrp",
        status: "ready",
    })))
}

pub async fn run(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<RunMrpRequest>,
) -> AppResult<Json<ApiResponse<RunMrpResponse>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let id_generator = PostgresMrpIdGenerator::default();

    let use_case = RunMrpUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        id_generator,
    );

    if request.demand_qty <= Decimal::ZERO {
        return Err(AppError::Business {
            code: "MRP_DEMAND_INVALID",
            message: "需求数量必须大于 0".to_string(),
        });
    }

    let variant_code = normalize_optional_string(request.variant_code);
    let finished_material_id = normalize_optional_string(request.finished_material_id);

    // 当前 v9 的 wms.fn_run_mrp() 按产品变体运行，
    // 所以 MVP 阶段必须提供 variant_code。
    if variant_code.is_none() {
        return Err(AppError::Business {
            code: "MRP_VARIANT_REQUIRED",
            message: "当前 MRP 运行必须提供 variant_code".to_string(),
        });
    }

    let command = RunMrpCommand {
        material_id: finished_material_id.map(MaterialId::new),
        product_variant_id: variant_code.map(ProductVariantId::new),
        demand_qty: request.demand_qty,
        demand_date: request.demand_date.with_time(Time::MIDNIGHT).assume_utc(),
        created_by: operator_from_headers(&headers),
        remark: normalize_optional_string(request.remark),
    };

    let output = use_case.execute(command).await?;

    Ok(Json(ApiResponse::ok(RunMrpResponse {
        run_id: output.run_id,
        status: output.status,
    })))
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

    let result = <PostgresMrpStore as MrpRunRepository>::list(&store, query).await?;

    Ok(Json(ApiResponse::ok(result)))
}


pub async fn get_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::MrpRun>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let run_id = MrpRunId::new(run_id);

    let run = <PostgresMrpStore as MrpRunRepository>::find_by_id(&store, &run_id)
        .await?
        .ok_or_else(|| cuba_shared::AppError::NotFound("MRP 运行记录不存在".to_string()))?;

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
    };

    let result = <PostgresMrpStore as MrpSuggestionRepository>::list(&store, query).await?;

    Ok(Json(ApiResponse::ok(result)))
}


pub async fn confirm_suggestion(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(suggestion_id): Path<String>,
    Json(request): Json<ConfirmMrpSuggestionRequest>,
) -> AppResult<Json<ApiResponse<ConfirmMrpSuggestionResponse>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let use_case = ConfirmMrpSuggestionUseCase::new(store);

    let output = use_case
        .execute(ConfirmMrpSuggestionCommand {
            suggestion_id: MrpSuggestionId::new(suggestion_id),
            confirmed_by: operator_from_headers(&headers),
            remark: request.remark,
        })
        .await?;

    Ok(Json(ApiResponse::ok(ConfirmMrpSuggestionResponse {
        suggestion_id: output.suggestion_id,
        status: output.status,
    })))
}


pub async fn cancel_suggestion(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(suggestion_id): Path<String>,
    Json(request): Json<CancelMrpSuggestionRequest>,
) -> AppResult<Json<ApiResponse<CancelMrpSuggestionResponse>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let use_case = CancelMrpSuggestionUseCase::new(store);

    let output = use_case
        .execute(CancelMrpSuggestionCommand {
            suggestion_id: MrpSuggestionId::new(suggestion_id),
            cancelled_by: operator_from_headers(&headers),
            reason: request.reason,
        })
        .await?;

    Ok(Json(ApiResponse::ok(CancelMrpSuggestionResponse {
        suggestion_id: output.suggestion_id,
        status: output.status,
    })))
}


pub async fn get_suggestion(
    State(state): State<AppState>,
    Path(suggestion_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::MrpSuggestion>>> {
    let store = PostgresMrpStore::new(state.db_pool.clone());
    let suggestion_id = MrpSuggestionId::new(suggestion_id);

    let suggestion =
        <PostgresMrpStore as MrpSuggestionRepository>::find_by_id(&store, &suggestion_id)
            .await?
            .ok_or_else(|| cuba_shared::AppError::NotFound("MRP 建议不存在".to_string()))?;

    Ok(Json(ApiResponse::ok(suggestion)))
}
