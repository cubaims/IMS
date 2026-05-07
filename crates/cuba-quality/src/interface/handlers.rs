use axum::{
    extract::{Path, Query, State},
    Json,
};
use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{
        AddInspectionResultUseCase, CreateInspectionLotUseCase, FreezeBatchUseCase,
        InspectionLotQuery, MakeInspectionDecisionUseCase,
    },
    domain::InspectionLotId,
    infrastructure::{PostgresQualityIdGenerator, PostgresQualityStore},
    interface::dto::{
        AddInspectionResultRequest, BatchAddInspectionResultsRequest, CreateInspectionLotRequest,
        FreezeBatchRequest, MakeInspectionDecisionRequest, ScrapBatchRequest, UnfreezeBatchRequest,
    },
};

fn service(state: &AppState) -> PostgresQualityStore {
    PostgresQualityStore::new(state.db_pool.clone())
}

fn quality_id_generator() -> PostgresQualityIdGenerator {
    PostgresQualityIdGenerator::default()
}

fn operator_from_headers(headers: &axum::http::HeaderMap) -> crate::domain::Operator {
    let username = headers
        .get("x-user-name")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("API")
        .to_string();
    crate::domain::Operator::new(username)
}

// =============================================================================
// 检验批
// =============================================================================
pub async fn create_inspection_lot_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreateInspectionLotRequest>,
) -> AppResult<Json<ApiResponse<crate::interface::dto::CreateInspectionLotResponse>>> {
    let operator = operator_from_headers(&headers);
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case = CreateInspectionLotUseCase::new(store.clone(), store.clone(), store.clone(), id_generator);

    let output = use_case.execute(request.into_command(operator)).await?;

    Ok(Json(ApiResponse::ok(crate::interface::dto::CreateInspectionLotResponse {
        inspection_lot_id: output.inspection_lot_id.as_str().to_string(),
        batch_number: output.batch_number.as_str().to_string(),
        batch_status_changed: output.batch_status_changed,
    })))
}

pub async fn list_inspection_lots_handler(
    State(state): State<AppState>,
    Query(query): Query<InspectionLotQuery>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let store = service(&state);
    let result = store.list(query).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_inspection_lot_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let store = service(&state);
    let lot = store.find_by_id(&InspectionLotId::new(lot_id)).await?;
    Ok(Json(ApiResponse::ok(lot)))
}

// =============================================================================
// 检验结果
// =============================================================================
pub async fn add_inspection_result_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(lot_id): Path<String>,
    Json(request): Json<AddInspectionResultRequest>,
) -> AppResult<Json<ApiResponse<crate::interface::dto::AddInspectionResultResponse>>> {
    let operator = operator_from_headers(&headers);
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case = AddInspectionResultUseCase::new(store.clone(), store.clone(), store.clone(), id_generator);

    let output = use_case.execute(request.into_command(lot_id, operator)).await?;

    Ok(Json(ApiResponse::ok(crate::interface::dto::AddInspectionResultResponse {
        result_id: output.result_id.as_str().to_string(),
        result_status: output.result_status,
    })))
}

pub async fn batch_add_inspection_results_handler(
    State(_state): State<AppState>,
    _headers: axum::http::HeaderMap,
    Path(_lot_id): Path<String>,
    Json(_request): Json<BatchAddInspectionResultsRequest>, // MVP 先占位，后续实现
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    // 后续可扩展为批量事务
    Ok(Json(ApiResponse::ok(serde_json::json!({"message": "batch add not implemented yet"}))))
}

pub async fn list_inspection_results_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let store = service(&state);
    let results = store.find_by_lot_id(&InspectionLotId::new(lot_id)).await?;
    Ok(Json(ApiResponse::ok(results)))
}

// =============================================================================
// 质量判定 & 批次操作（核心 4 个）
// =============================================================================
pub async fn make_inspection_decision_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(lot_id): Path<String>,
    Json(request): Json<MakeInspectionDecisionRequest>,
) -> AppResult<Json<ApiResponse<crate::interface::dto::MakeInspectionDecisionResponse>>> {
    let operator = operator_from_headers(&headers);
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case = MakeInspectionDecisionUseCase::new(
        store.clone(), store.clone(), store.clone(), store.clone(), store.clone(), id_generator,
    );

    let output = use_case.execute(request.into_command(lot_id, operator)).await?;

    Ok(Json(ApiResponse::ok(crate::interface::dto::MakeInspectionDecisionResponse {
        inspection_lot_id: output.inspection_lot_id.as_str().to_string(),
        decision: output.decision,
        notification_id: output.notification_id.map(|id| id.as_str().to_string()),
    })))
}

pub async fn freeze_batch_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(batch_number): Path<String>,
    Json(request): Json<FreezeBatchRequest>,
) -> AppResult<Json<ApiResponse<crate::interface::dto::BatchActionResponse>>> {
    let operator = operator_from_headers(&headers);
    let store = service(&state);
    let use_case = FreezeBatchUseCase::new(store);

    let output = use_case.execute(request.into_command(batch_number, operator)).await?;

    Ok(Json(ApiResponse::ok(crate::interface::dto::BatchActionResponse {
        batch_number: output.batch_number.as_str().to_string(),
        success: true,
    })))
}

// unfreeze / scrap / status / history 暂时保持原样（后续可按此模板补全）
pub async fn unfreeze_batch_handler(_state: State<AppState>, _headers: axum::http::HeaderMap, _path: Path<String>, _req: Json<UnfreezeBatchRequest>) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    Ok(Json(ApiResponse::ok(serde_json::json!({"message": "unfreeze not implemented yet"}))))
}
pub async fn scrap_batch_handler(_state: State<AppState>, _headers: axum::http::HeaderMap, _path: Path<String>, _req: Json<ScrapBatchRequest>) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    Ok(Json(ApiResponse::ok(serde_json::json!({"message": "scrap not implemented yet"}))))
}
pub async fn get_batch_quality_status_handler(_state: State<AppState>, _path: Path<String>) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    Ok(Json(ApiResponse::ok(serde_json::json!({"message": "status not implemented yet"}))))
}
pub async fn list_batch_quality_history_handler(_state: State<AppState>, _path: Path<String>, _query: Query<crate::application::BatchHistoryQuery>) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    Ok(Json(ApiResponse::ok(serde_json::json!({"message": "history not implemented yet"}))))
}
pub async fn list_quality_notifications_handler(
    State(_state): State<AppState>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": [],
        "message": "quality notification list not implemented yet"
    }))))
}

pub async fn get_quality_notification_handler(
    State(_state): State<AppState>,
    Path(notification_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "notification_id": notification_id,
        "message": "quality notification detail not implemented yet"
    }))))
}
