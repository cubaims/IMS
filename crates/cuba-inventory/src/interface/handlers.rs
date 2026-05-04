use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};

use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{BatchRepository, InventoryRepository, InventoryService, MapHistoryRepository},
    infrastructure::PostgresInventoryRepository,
    interface::dto::{
        BatchHistoryRequest, BatchRequest, CurrentStockRequest, InventoryTransactionRequest,
        MapHistoryRequest, PickBatchFefoRequest, PostInventoryRequest, TransferInventoryRequest,
    },
};

fn build_service(state: &AppState) -> InventoryService {
    let repo = Arc::new(PostgresInventoryRepository::new(state.db_pool.clone()));

    let inventory_repo: Arc<dyn InventoryRepository> = repo.clone();
    let batch_repo: Arc<dyn BatchRepository> = repo.clone();
    let map_history_repo: Arc<dyn MapHistoryRepository> = repo;

    InventoryService::new(inventory_repo, batch_repo, map_history_repo)
}

fn operator_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-user-name")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("API")
        .to_string()
}

pub async fn post_inventory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PostInventoryRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryPostingResult>>> {
    let service = build_service(&state);
    let operator = operator_from_headers(&headers);

    let result = service.post_inventory(request.into(), operator).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn transfer_inventory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TransferInventoryRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryPostingResult>>> {
    let service = build_service(&state);
    let operator = operator_from_headers(&headers);

    let result = service.transfer_inventory(request.into(), operator).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn pick_batch_fefo(
    State(state): State<AppState>,
    Json(request): Json<PickBatchFefoRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = build_service(&state);

    let result = service.pick_batch_fefo(request.into()).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_current_stock(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::CurrentStock>>>> {
    let service = build_service(&state);

    let result = service.list_current_stock(query.into()).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_bin_stock(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BinStock>>>> {
    let service = build_service(&state);

    let result = service.list_bin_stock(query.into()).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_transactions(
    State(state): State<AppState>,
    Query(query): Query<InventoryTransactionRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::InventoryTransaction>>>> {
    let service = build_service(&state);

    let result = service.list_transactions(query.into()).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_transaction(
    State(state): State<AppState>,
    Path(transaction_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryTransaction>>> {
    let service = build_service(&state);

    let result = service.get_transaction(transaction_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_batches(
    State(state): State<AppState>,
    Query(query): Query<BatchRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::Batch>>>> {
    let service = build_service(&state);

    let result = service.list_batches(query.into()).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_batch(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::Batch>>> {
    let service = build_service(&state);

    let result = service.get_batch(batch_number).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_batch_history(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Query(query): Query<BatchHistoryRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BatchHistory>>>> {
    let service = build_service(&state);

    let result = service
        .list_batch_history(batch_number, query.into())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_map_history(
    State(state): State<AppState>,
    Query(query): Query<MapHistoryRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::MapHistory>>>> {
    let service = build_service(&state);

    let result = service.list_map_history(query.into()).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_material_map_history(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
    Query(query): Query<MapHistoryRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::MapHistory>>>> {
    let service = build_service(&state);

    let result = service
        .list_material_map_history(material_id, query.into())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}
