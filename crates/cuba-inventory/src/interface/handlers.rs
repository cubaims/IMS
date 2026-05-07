use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;
use std::sync::Arc;

use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{
        BatchRepository, InventoryRepository, InventoryService, MapHistoryRepository,
        inventory_count_service::InventoryCountService,
    },
    infrastructure::{PostgresInventoryCountRepository, PostgresInventoryRepository},
    interface::dto::{
        // 盘点 DTO
        ApproveInventoryCountRequest,
        // 库存核心 DTO
        BatchHistoryRequest,
        BatchRequest,
        BatchUpdateInventoryCountLinesRequest,
        CancelInventoryCountRequest,
        CloseInventoryCountRequest,
        CreateInventoryCountRequest,
        CurrentStockRequest,
        InventoryCountLineResponse,
        InventoryCountResponse,
        InventoryTransactionRequest,
        MapHistoryRequest,
        PickBatchFefoRequest,
        PostInventoryCountRequest,
        PostInventoryRequest,
        SubmitInventoryCountRequest,
        TransferInventoryRequest,
        UpdateInventoryCountLineRequest,
    },
};

// ==================== 库存核心 service ====================
fn inventory_service(state: &AppState) -> InventoryService {
    let repo = Arc::new(PostgresInventoryRepository::new(state.db_pool.clone()));
    let inventory_repo: Arc<dyn InventoryRepository> = repo.clone();
    let batch_repo: Arc<dyn BatchRepository> = repo.clone();
    let map_history_repo: Arc<dyn MapHistoryRepository> = repo;

    InventoryService::new(inventory_repo, batch_repo, map_history_repo)
}

// ==================== 盘点模块 service ====================
fn count_service(state: &AppState) -> InventoryCountService<PostgresInventoryCountRepository> {
    let repo = Arc::new(PostgresInventoryCountRepository::new(state.db_pool.clone()));
    InventoryCountService::new(repo)
}

fn operator_from_headers(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-user-name")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("API")
        .to_string()
}

// ====================== 库存核心功能 ======================
pub async fn post_inventory(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<PostInventoryRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryPostingResult>>> {
    let service = inventory_service(&state);
    let operator = operator_from_headers(&headers);
    let result = service.post_inventory(request.into(), operator).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn transfer_inventory(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<TransferInventoryRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryPostingResult>>> {
    let service = inventory_service(&state);
    let operator = operator_from_headers(&headers);
    let result = service.transfer_inventory(request.into(), operator).await?;
    Ok(Json(ApiResponse::ok(result)))
}

// 查询接口（保持不变）
pub async fn list_current_stock(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::CurrentStock>>>> {
    let service = inventory_service(&state);
    let result = service.list_current_stock(query.into()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_bin_stock(
    State(state): State<AppState>,
    Query(query): Query<CurrentStockRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BinStock>>>> {
    let service = inventory_service(&state);
    let result = service.list_bin_stock(query.into()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_transactions(
    State(state): State<AppState>,
    Query(query): Query<InventoryTransactionRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::InventoryTransaction>>>> {
    let service = inventory_service(&state);
    let result = service.list_transactions(query.into()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_transaction(
    State(state): State<AppState>,
    Path(transaction_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryTransaction>>> {
    let service = inventory_service(&state);
    let result = service.get_transaction(transaction_id).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_batches(
    State(state): State<AppState>,
    Query(query): Query<BatchRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::Batch>>>> {
    let service = inventory_service(&state);
    let result = service.list_batches(query.into()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_batch(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::Batch>>> {
    let service = inventory_service(&state);
    let result = service.get_batch(batch_number).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_batch_history(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
    Query(query): Query<BatchHistoryRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BatchHistory>>>> {
    let service = inventory_service(&state);
    let result = service
        .list_batch_history(batch_number, query.into())
        .await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_map_history(
    State(state): State<AppState>,
    Query(query): Query<MapHistoryRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::MapHistory>>>> {
    let service = inventory_service(&state);
    let result = service.list_map_history(query.into()).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_material_map_history(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
    Query(query): Query<MapHistoryRequest>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::MapHistory>>>> {
    let service = inventory_service(&state);
    let result = service
        .list_material_map_history(material_id, query.into())
        .await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn pick_batch_fefo(
    State(state): State<AppState>,
    Json(request): Json<PickBatchFefoRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let service = inventory_service(&state);
    let result = service.pick_batch_fefo(request.into()).await?;
    Ok(Json(ApiResponse::ok(result)))
}
// ====================== 盘点模块功能（真正实现 + 错误转换） ======================
pub async fn create_inventory_count(
    State(state): State<AppState>,
    Json(req): Json<CreateInventoryCountRequest>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service.create_count(req.into()).await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn list_inventory_counts(
    State(state): State<AppState>,
    Query(query): Query<crate::application::ListInventoryCountsInput>,
) -> AppResult<
    Json<ApiResponse<crate::application::common::Page<crate::application::InventoryCountSummary>>>,
> {
    let service = count_service(&state);
    let result = service.list_counts(query).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_inventory_count(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service
        .get_count(crate::application::GetInventoryCountInput { count_doc_id })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn generate_inventory_count_lines(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service
        .generate_lines(crate::application::GenerateInventoryCountLinesInput {
            count_doc_id,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn update_inventory_count_line(
    State(state): State<AppState>,
    Path((count_doc_id, line_no)): Path<(String, i32)>,
    Json(req): Json<UpdateInventoryCountLineRequest>,
) -> AppResult<Json<ApiResponse<InventoryCountLineResponse>>> {
    let service = count_service(&state);
    let result = service
        .update_line(crate::application::UpdateInventoryCountLineInput {
            count_doc_id,
            line_no,
            counted_qty: req.counted_qty,
            difference_reason: req.difference_reason,
            remark: req.remark,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn batch_update_inventory_count_lines(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
    Json(req): Json<BatchUpdateInventoryCountLinesRequest>,
) -> AppResult<Json<ApiResponse<Vec<InventoryCountLineResponse>>>> {
    let service = count_service(&state);

    // 正确转换 DTO -> application model
    let lines: Vec<crate::application::inventory_count_model::BatchUpdateInventoryCountLineItem> =
        req.lines
            .into_iter()
            .map(|item| crate::application::inventory_count_model::BatchUpdateInventoryCountLineItem {
                line_no: item.line_no,
                counted_qty: item.counted_qty,
                difference_reason: item.difference_reason,
                remark: item.remark,
            })
            .collect();

    let result = service
        .batch_update_lines(crate::application::BatchUpdateInventoryCountLinesInput {
            count_doc_id,
            lines,
            operator: "system".to_string(),
        })
        .await?;

    let response: Vec<InventoryCountLineResponse> = result
        .into_iter()
        .map(InventoryCountLineResponse::from)
        .collect();

    Ok(Json(ApiResponse::ok(response)))
}

pub async fn submit_inventory_count(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
    Json(req): Json<SubmitInventoryCountRequest>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service
        .submit(crate::application::SubmitInventoryCountInput {
            count_doc_id,
            remark: req.remark,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn approve_inventory_count(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
    Json(req): Json<ApproveInventoryCountRequest>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service
        .approve(crate::application::ApproveInventoryCountInput {
            count_doc_id,
            approved: req.approved,
            remark: req.remark,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn post_inventory_count(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
    Json(req): Json<PostInventoryCountRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::InventoryCountPostingResult>>> {
    let service = count_service(&state);
    let result = service
        .post(crate::application::PostInventoryCountInput {
            count_doc_id,
            posting_date: req.posting_date,
            remark: req.remark,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn close_inventory_count(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
    Json(req): Json<CloseInventoryCountRequest>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service
        .close(crate::application::CloseInventoryCountInput {
            count_doc_id,
            remark: req.remark,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}

pub async fn cancel_inventory_count(
    State(state): State<AppState>,
    Path(count_doc_id): Path<String>,
    Json(req): Json<CancelInventoryCountRequest>,
) -> AppResult<Json<ApiResponse<InventoryCountResponse>>> {
    let service = count_service(&state);
    let result = service
        .cancel(crate::application::CancelInventoryCountInput {
            count_doc_id,
            remark: req.remark,
            operator: "system".to_string(),
        })
        .await?;
    Ok(Json(ApiResponse::ok(result.into())))
}
