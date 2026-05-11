use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use cuba_shared::{
    ApiResponse, AppError, AppResult, AppState, CurrentUser, Page, write_audit_event,
};

use crate::{
    application::{
        AddInspectionResultUseCase, BatchAddInspectionResultsUseCase, BatchQualityRepository,
        BatchQualityStatusView, CreateInspectionLotUseCase, FreezeBatchUseCase,
        InspectionLotRepository, InspectionLotSummary, InspectionResultRepository,
        MakeInspectionDecisionUseCase, QualityNotificationRepository, QualityNotificationSummary,
        ScrapBatchUseCase, UnfreezeBatchUseCase,
    },
    domain::{
        BatchNumber, BatchQualityHistory, InspectionLot, InspectionLotId, InspectionResult,
        QualityNotification, QualityNotificationId,
    },
    infrastructure::{PostgresQualityIdGenerator, PostgresQualityStore},
    interface::dto::{
        AddInspectionResultRequest, AddInspectionResultResponse, BatchActionResponse,
        BatchAddInspectionResultsRequest, BatchAddInspectionResultsResponse, BatchHistoryListQuery,
        CreateInspectionLotRequest, CreateInspectionLotResponse, FreezeBatchRequest,
        InspectionLotListQuery, MakeInspectionDecisionRequest, MakeInspectionDecisionResponse,
        QualityNotificationListQuery, ScrapBatchRequest, UnfreezeBatchRequest,
    },
};

fn service(state: &AppState) -> PostgresQualityStore {
    PostgresQualityStore::new(state.db_pool.clone())
}

fn quality_id_generator() -> PostgresQualityIdGenerator {
    PostgresQualityIdGenerator::default()
}

// =============================================================================
// 检验批
// =============================================================================
pub async fn create_inspection_lot_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<CreateInspectionLotRequest>,
) -> AppResult<Json<ApiResponse<CreateInspectionLotResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case =
        CreateInspectionLotUseCase::new(store.clone(), store.clone(), store.clone(), id_generator);

    let output = use_case.execute(request.into_command(operator)).await?;
    let inspection_lot_id = output.inspection_lot_id.as_str().to_string();
    let batch_number = output.batch_number.as_str().to_string();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_INSPECTION_LOT_CREATE",
        Some("wms.wms_inspection_lots"),
        Some(&inspection_lot_id),
        Some(serde_json::json!({
            "inspection_lot_id": inspection_lot_id,
            "batch_number": batch_number,
            "batch_status_changed": output.batch_status_changed
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(CreateInspectionLotResponse {
        inspection_lot_id,
        batch_number,
        batch_status_changed: output.batch_status_changed,
    })))
}

pub async fn list_inspection_lots_handler(
    State(state): State<AppState>,
    Query(query): Query<InspectionLotListQuery>,
) -> AppResult<Json<ApiResponse<Page<InspectionLotSummary>>>> {
    let store = service(&state);
    let result = InspectionLotRepository::list(&store, query.into_application_query()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_inspection_lot_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
) -> AppResult<Json<ApiResponse<InspectionLot>>> {
    let store = service(&state);
    let lot_id = InspectionLotId::new(lot_id);
    let lot = InspectionLotRepository::find_by_id(&store, &lot_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("检验批不存在: {}", lot_id.as_str())))?;
    Ok(Json(ApiResponse::ok(lot)))
}

// =============================================================================
// 检验结果
// =============================================================================
pub async fn add_inspection_result_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(lot_id): Path<String>,
    Json(request): Json<AddInspectionResultRequest>,
) -> AppResult<Json<ApiResponse<AddInspectionResultResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case =
        AddInspectionResultUseCase::new(store.clone(), store.clone(), store.clone(), id_generator);

    let output = use_case
        .execute(request.into_command(lot_id.clone(), operator))
        .await?;
    let result_id = output.result_id.as_str().to_string();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_INSPECTION_RESULT_ADD",
        Some("wms.wms_inspection_results"),
        Some(&result_id),
        Some(serde_json::json!({
            "inspection_lot_id": lot_id,
            "result_id": result_id,
            "result_status": output.result_status
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(AddInspectionResultResponse {
        result_id,
        result_status: output.result_status,
    })))
}

pub async fn batch_add_inspection_results_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(lot_id): Path<String>,
    Json(request): Json<BatchAddInspectionResultsRequest>,
) -> AppResult<Json<ApiResponse<BatchAddInspectionResultsResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case = BatchAddInspectionResultsUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        id_generator,
    );

    let output = use_case
        .execute(request.into_command(lot_id.clone(), operator))
        .await?;

    let results: Vec<AddInspectionResultResponse> = output
        .results
        .into_iter()
        .map(|result| AddInspectionResultResponse {
            result_id: result.result_id.as_str().to_string(),
            result_status: result.result_status,
        })
        .collect();

    let result_ids: Vec<String> = results
        .iter()
        .map(|result| result.result_id.clone())
        .collect();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_INSPECTION_RESULT_BATCH_ADD",
        Some("wms.wms_inspection_results"),
        Some(&lot_id),
        Some(serde_json::json!({
            "inspection_lot_id": lot_id,
            "result_ids": result_ids
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(BatchAddInspectionResultsResponse {
        results,
    })))
}

pub async fn list_inspection_results_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
) -> AppResult<Json<ApiResponse<Vec<InspectionResult>>>> {
    let store = service(&state);
    let results =
        InspectionResultRepository::find_by_lot_id(&store, &InspectionLotId::new(lot_id)).await?;
    Ok(Json(ApiResponse::ok(results)))
}

// =============================================================================
// 质量判定 & 批次操作（核心 4 个）
// =============================================================================
pub async fn make_inspection_decision_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(lot_id): Path<String>,
    Json(request): Json<MakeInspectionDecisionRequest>,
) -> AppResult<Json<ApiResponse<MakeInspectionDecisionResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let id_generator = quality_id_generator();

    let use_case = MakeInspectionDecisionUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
        id_generator,
    );

    let output = use_case
        .execute(request.into_command(lot_id, operator))
        .await?;
    let inspection_lot_id = output.inspection_lot_id.as_str().to_string();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_DECISION",
        Some("wms.wms_inspection_lots"),
        Some(&inspection_lot_id),
        Some(serde_json::json!({
            "inspection_lot_id": inspection_lot_id,
            "decision": output.decision,
            "notification_id": output.notification_id.as_ref().map(|id| id.as_str().to_string())
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(MakeInspectionDecisionResponse {
        inspection_lot_id,
        decision: output.decision,
        notification_id: output.notification_id.map(|id| id.as_str().to_string()),
    })))
}

pub async fn freeze_batch_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(batch_number): Path<String>,
    Json(request): Json<FreezeBatchRequest>,
) -> AppResult<Json<ApiResponse<BatchActionResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let use_case = FreezeBatchUseCase::new(store);

    let output = use_case
        .execute(request.into_command(batch_number, operator))
        .await?;
    let batch_number = output.batch_number.as_str().to_string();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_BATCH_FREEZE",
        Some("wms.wms_batches"),
        Some(&batch_number),
        Some(serde_json::json!({ "batch_number": batch_number })),
    )
    .await;

    Ok(Json(ApiResponse::ok(BatchActionResponse {
        batch_number,
        success: true,
    })))
}

pub async fn unfreeze_batch_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(batch_number): Path<String>,
    Json(request): Json<UnfreezeBatchRequest>,
) -> AppResult<Json<ApiResponse<BatchActionResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let use_case = UnfreezeBatchUseCase::new(store);
    let target_status = request.target_status;

    let output = use_case
        .execute(request.into_command(batch_number, operator))
        .await?;
    let batch_number = output.batch_number.as_str().to_string();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_BATCH_UNFREEZE",
        Some("wms.wms_batches"),
        Some(&batch_number),
        Some(serde_json::json!({
            "batch_number": batch_number,
            "target_status": target_status
        })),
    )
    .await;

    Ok(Json(ApiResponse::ok(BatchActionResponse {
        batch_number,
        success: true,
    })))
}
pub async fn scrap_batch_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(batch_number): Path<String>,
    Json(request): Json<ScrapBatchRequest>,
) -> AppResult<Json<ApiResponse<BatchActionResponse>>> {
    let operator = crate::domain::Operator::new(user.username.clone());
    let store = service(&state);
    let use_case = ScrapBatchUseCase::new(store.clone(), store);

    let output = use_case
        .execute(request.into_command(batch_number, operator))
        .await?;
    let batch_number = output.batch_number.as_str().to_string();
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "QUALITY_BATCH_SCRAP",
        Some("wms.wms_batches"),
        Some(&batch_number),
        Some(serde_json::json!({ "batch_number": batch_number })),
    )
    .await;

    Ok(Json(ApiResponse::ok(BatchActionResponse {
        batch_number,
        success: true,
    })))
}
pub async fn get_batch_quality_status_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<BatchQualityStatusView>>> {
    let store = service(&state);
    let batch_number = BatchNumber::new(batch_number);
    let result = BatchQualityRepository::get_batch_status(&store, &batch_number).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_batch_quality_history_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Query(query): Query<BatchHistoryListQuery>,
) -> AppResult<Json<ApiResponse<Page<BatchQualityHistory>>>> {
    let store = service(&state);
    let batch_number = BatchNumber::new(batch_number);
    let result = BatchQualityRepository::list_batch_history(
        &store,
        &batch_number,
        query.into_application_query(),
    )
    .await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_quality_notifications_handler(
    State(state): State<AppState>,
    Query(query): Query<QualityNotificationListQuery>,
) -> AppResult<Json<ApiResponse<Page<QualityNotificationSummary>>>> {
    let store = service(&state);
    let result =
        QualityNotificationRepository::list(&store, query.into_application_query()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_quality_notification_handler(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
) -> AppResult<Json<ApiResponse<QualityNotification>>> {
    let store = service(&state);
    let notification_id = QualityNotificationId::new(notification_id);
    let notification = QualityNotificationRepository::find_by_id(&store, &notification_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("质量通知不存在: {}", notification_id.as_str()))
        })?;

    Ok(Json(ApiResponse::ok(notification)))
}
