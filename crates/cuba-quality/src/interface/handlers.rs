use crate::application::{
    AddInspectionResultUseCase, BatchHistoryQuery, CreateInspectionLotUseCase,
    FreezeBatchUseCase, InspectionLotQuery, InspectionLotRepository,
    InspectionResultRepository, MakeInspectionDecisionUseCase,
    QualityNotificationQuery, QualityNotificationRepository, ScrapBatchUseCase,
    UnfreezeBatchUseCase,
};
use crate::domain::{
    BatchNumber, InspectionLotId, QualityError, QualityNotificationId,
};
use crate::infrastructure::{
    PostgresQualityIdGenerator, PostgresQualityStore,
};
use crate::interface::{
    AddInspectionResultRequest, AddInspectionResultResponse,
    BatchActionResponse, BatchAddInspectionResultsRequest,
    BatchAddInspectionResultsResponse, CreateInspectionLotRequest,
    CreateInspectionLotResponse, CurrentUser, FreezeBatchRequest,
    MakeInspectionDecisionRequest, MakeInspectionDecisionResponse,
    ScrapBatchRequest, UnfreezeBatchRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use cuba_shared::{AppState, PageQuery};
use serde::{Deserialize, Serialize};

/// 从全局 AppState 构造质量模块 PostgreSQL Store。
///
/// 这样质量模块不用自己维护单独的 State，
/// 统一复用 cuba-api 创建好的 PgPool。
fn quality_store(state: &AppState) -> PostgresQualityStore {
    PostgresQualityStore::new(state.db_pool.clone())
}

/// 获取质量模块 ID 生成器。
///
/// 当前是 UUID 版本。
/// 后面如果改成数据库流水号，也可以在这里替换。
fn quality_id_generator() -> PostgresQualityIdGenerator {
    PostgresQualityIdGenerator::default()
}

/// 统一 API 响应。
///
/// 后续可以迁移到 cuba-shared/src/response.rs。
#[derive(Debug, Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: Option<T>,
    pub error_code: Option<String>,
    pub message: String,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error_code: None,
            message: "OK".to_string(),
        }
    }
}

/// API 错误响应。
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub error_code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(
        status: StatusCode,
        error_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            error_code: error_code.into(),
            message: message.into(),
        }
    }
}

impl From<QualityError> for ApiError {
    fn from(error: QualityError) -> Self {
        match error {
            QualityError::InspectionLotNotFound => Self::new(
                StatusCode::NOT_FOUND,
                "INSPECTION_LOT_NOT_FOUND",
                error.to_string(),
            ),
            QualityError::QualityNotificationNotFound => Self::new(
                StatusCode::NOT_FOUND,
                "QUALITY_NOTIFICATION_NOT_FOUND",
                error.to_string(),
            ),
            QualityError::InspectionCharNotFound => Self::new(
                StatusCode::NOT_FOUND,
                "INSPECTION_CHAR_NOT_FOUND",
                error.to_string(),
            ),
            QualityError::DefectCodeNotFound => Self::new(
                StatusCode::NOT_FOUND,
                "DEFECT_CODE_NOT_FOUND",
                error.to_string(),
            ),
            QualityError::InspectionLotStatusInvalid => Self::new(
                StatusCode::CONFLICT,
                "INSPECTION_LOT_STATUS_INVALID",
                error.to_string(),
            ),
            QualityError::BatchAlreadyFrozen => Self::new(
                StatusCode::CONFLICT,
                "BATCH_ALREADY_FROZEN",
                error.to_string(),
            ),
            QualityError::BatchNotFrozen => Self::new(
                StatusCode::CONFLICT,
                "BATCH_NOT_FROZEN",
                error.to_string(),
            ),
            QualityError::BatchAlreadyScrapped => Self::new(
                StatusCode::CONFLICT,
                "BATCH_ALREADY_SCRAPPED",
                error.to_string(),
            ),
            QualityError::BatchPendingInspection => Self::new(
                StatusCode::CONFLICT,
                "BATCH_PENDING_INSPECTION",
                error.to_string(),
            ),
            QualityError::BatchFrozen => Self::new(
                StatusCode::CONFLICT,
                "BATCH_FROZEN",
                error.to_string(),
            ),
            QualityError::BatchScrapped => Self::new(
                StatusCode::CONFLICT,
                "BATCH_SCRAPPED",
                error.to_string(),
            ),
            QualityError::RequiredFieldEmpty(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "REQUIRED_FIELD_EMPTY",
                error.to_string(),
            ),
            QualityError::QuantityMustBePositive => Self::new(
                StatusCode::BAD_REQUEST,
                "QUANTITY_MUST_BE_POSITIVE",
                error.to_string(),
            ),
            QualityError::SampleQtyExceeded => Self::new(
                StatusCode::BAD_REQUEST,
                "SAMPLE_QTY_EXCEEDED",
                error.to_string(),
            ),
            _ => Self::new(
                StatusCode::BAD_REQUEST,
                "QUALITY_ERROR",
                error.to_string(),
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiResponse::<()> {
            success: false,
            data: None,
            error_code: Some(self.error_code),
            message: self.message,
        };

        (self.status, Json(body)).into_response()
    }
}

/// MVP 当前用户。
///
/// 先固定为 SYSTEM。
/// 后面接入 cuba-auth 后，可以从 JWT middleware 里取当前用户。
fn current_user_from_mvp() -> CurrentUser {
    CurrentUser {
        username: "SYSTEM".to_string(),
    }
}

// =============================================================================
// 查询参数
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct PageOnlyQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

impl PageOnlyQuery {
    pub fn into_page_query(self) -> PageQuery {
        PageQuery {
            page: self.page.unwrap_or(1),
            page_size: self.page_size.unwrap_or(20),
        }
    }
}

// =============================================================================
// 检验批 Handlers
// =============================================================================

/// 创建检验批。
///
/// POST /api/quality/inspection-lots
pub async fn create_inspection_lot_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateInspectionLotRequest>,
) -> Result<Json<ApiResponse<CreateInspectionLotResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);
    let id_generator = quality_id_generator();

    let use_case = CreateInspectionLotUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        id_generator,
    );

    let output = use_case
        .execute(payload.into_command(user.operator()))
        .await?;

    Ok(Json(ApiResponse::ok(CreateInspectionLotResponse {
        inspection_lot_id: output.inspection_lot_id.as_str().to_string(),
        batch_number: output.batch_number.as_str().to_string(),
        batch_status_changed: output.batch_status_changed,
    })))
}

/// 查询检验批详情。
///
/// GET /api/quality/inspection-lots/{lot_id}
pub async fn get_inspection_lot_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let lot = <PostgresQualityStore as InspectionLotRepository>::find_by_id(
        &store,
        &InspectionLotId::new(lot_id),
    )
        .await?
        .ok_or(QualityError::InspectionLotNotFound)?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(lot).unwrap_or_else(|_| serde_json::json!({})),
    )))
}

/// 检验批列表。
///
/// GET /api/quality/inspection-lots?page=1&page_size=20
pub async fn list_inspection_lots_handler(
    State(state): State<AppState>,
    Query(query): Query<PageOnlyQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let result = <PostgresQualityStore as InspectionLotRepository>::list(
        &store,
        InspectionLotQuery {
            page: query.into_page_query(),
            lot_type: None,
            status: None,
            material_id: None,
            batch_number: None,
            date_from: None,
            date_to: None,
        },
    )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({})),
    )))
}

// =============================================================================
// 检验结果 Handlers
// =============================================================================

/// 录入单条检验结果。
///
/// POST /api/quality/inspection-lots/{lot_id}/results
pub async fn add_inspection_result_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
    Json(payload): Json<AddInspectionResultRequest>,
) -> Result<Json<ApiResponse<AddInspectionResultResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);
    let id_generator = quality_id_generator();

    let use_case = AddInspectionResultUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        id_generator,
    );

    let output = use_case
        .execute(payload.into_command(lot_id, user.operator()))
        .await?;

    Ok(Json(ApiResponse::ok(AddInspectionResultResponse {
        result_id: output.result_id.as_str().to_string(),
        result_status: output.result_status,
    })))
}

/// 批量录入检验结果。
///
/// POST /api/quality/inspection-lots/{lot_id}/results/batch
pub async fn batch_add_inspection_results_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
    Json(payload): Json<BatchAddInspectionResultsRequest>,
) -> Result<Json<ApiResponse<BatchAddInspectionResultsResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);
    let id_generator = quality_id_generator();

    let use_case = AddInspectionResultUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        id_generator,
    );

    let mut results = Vec::with_capacity(payload.results.len());

    for item in payload.results {
        let output = use_case
            .execute(item.into_command(lot_id.clone(), user.operator()))
            .await?;

        results.push(AddInspectionResultResponse {
            result_id: output.result_id.as_str().to_string(),
            result_status: output.result_status,
        });
    }

    Ok(Json(ApiResponse::ok(BatchAddInspectionResultsResponse {
        results,
    })))
}

/// 查询检验批结果列表。
///
/// GET /api/quality/inspection-lots/{lot_id}/results
pub async fn list_inspection_results_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let results = <PostgresQualityStore as InspectionResultRepository>::find_by_lot_id(
        &store,
        &InspectionLotId::new(lot_id),
    )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(results).unwrap_or_else(|_| serde_json::json!([])),
    )))
}

// =============================================================================
// 质量判定 Handler
// =============================================================================

/// 质量判定。
///
/// POST /api/quality/inspection-lots/{lot_id}/decision
pub async fn make_inspection_decision_handler(
    State(state): State<AppState>,
    Path(lot_id): Path<String>,
    Json(payload): Json<MakeInspectionDecisionRequest>,
) -> Result<Json<ApiResponse<MakeInspectionDecisionResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);
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
        .execute(payload.into_command(lot_id, user.operator()))
        .await?;

    Ok(Json(ApiResponse::ok(MakeInspectionDecisionResponse {
        inspection_lot_id: output.inspection_lot_id.as_str().to_string(),
        decision: output.decision,
        notification_id: output.notification_id.map(|id| id.as_str().to_string()),
    })))
}

// =============================================================================
// 批次质量操作 Handlers
// =============================================================================

/// 冻结批次。
///
/// POST /api/quality/batches/{batch_number}/freeze
pub async fn freeze_batch_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Json(payload): Json<FreezeBatchRequest>,
) -> Result<Json<ApiResponse<BatchActionResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);
    let use_case = FreezeBatchUseCase::new(store);

    let output = use_case
        .execute(payload.into_command(batch_number, user.operator()))
        .await?;

    Ok(Json(ApiResponse::ok(BatchActionResponse {
        batch_number: output.batch_number.as_str().to_string(),
        success: true,
    })))
}

/// 解冻批次。
///
/// POST /api/quality/batches/{batch_number}/unfreeze
pub async fn unfreeze_batch_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Json(payload): Json<UnfreezeBatchRequest>,
) -> Result<Json<ApiResponse<BatchActionResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);
    let use_case = UnfreezeBatchUseCase::new(store);

    let output = use_case
        .execute(payload.into_command(batch_number, user.operator()))
        .await?;

    Ok(Json(ApiResponse::ok(BatchActionResponse {
        batch_number: output.batch_number.as_str().to_string(),
        success: true,
    })))
}

/// 质量报废批次。
///
/// POST /api/quality/batches/{batch_number}/scrap
pub async fn scrap_batch_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Json(payload): Json<ScrapBatchRequest>,
) -> Result<Json<ApiResponse<BatchActionResponse>>, ApiError> {
    let user = current_user_from_mvp();

    let store = quality_store(&state);

    let use_case = ScrapBatchUseCase::new(
        store.clone(),
        store.clone(),
    );

    let output = use_case
        .execute(payload.into_command(batch_number, user.operator()))
        .await?;

    Ok(Json(ApiResponse::ok(BatchActionResponse {
        batch_number: output.batch_number.as_str().to_string(),
        success: true,
    })))
}

/// 查询批次质量状态。
///
/// GET /api/quality/batches/{batch_number}/status
pub async fn get_batch_quality_status_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let status = store
        .get_batch_status(&BatchNumber::new(batch_number))
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(status).unwrap_or_else(|_| serde_json::json!({})),
    )))
}

/// 查询批次质量历史。
///
/// GET /api/quality/batches/{batch_number}/history?page=1&page_size=20
pub async fn list_batch_quality_history_handler(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Query(query): Query<PageOnlyQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let page = store
        .list_batch_history(
            &BatchNumber::new(batch_number),
            BatchHistoryQuery {
                page: query.into_page_query(),
            },
        )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(page).unwrap_or_else(|_| serde_json::json!({})),
    )))
}

// =============================================================================
// 质量通知 Handlers
// =============================================================================

/// 查询质量通知列表。
///
/// GET /api/quality/notifications?page=1&page_size=20
pub async fn list_quality_notifications_handler(
    State(state): State<AppState>,
    Query(query): Query<PageOnlyQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let page = <PostgresQualityStore as QualityNotificationRepository>::list(
        &store,
        QualityNotificationQuery {
            page: query.into_page_query(),
            status: None,
            severity: None,
            material_id: None,
            batch_number: None,
            owner: None,
            date_from: None,
            date_to: None,
        },
    )
        .await?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(page).unwrap_or_else(|_| serde_json::json!({})),
    )))
}

/// 查询质量通知详情。
///
/// GET /api/quality/notifications/{notification_id}
pub async fn get_quality_notification_handler(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let store = quality_store(&state);

    let notification =
        <PostgresQualityStore as QualityNotificationRepository>::find_by_id(
            &store,
            &QualityNotificationId::new(notification_id),
        )
            .await?
            .ok_or(QualityError::QualityNotificationNotFound)?;

    Ok(Json(ApiResponse::ok(
        serde_json::to_value(notification).unwrap_or_else(|_| serde_json::json!({})),
    )))
}