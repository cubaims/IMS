use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::InventoryResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "ready" })))
}

pub async fn transactions(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "transactions" })))
}


pub async fn current_stock(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "current-stock" })))
}


pub async fn bin_stock(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "bin-stock" })))
}


pub async fn batch_stock(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "batch-stock" })))
}


pub async fn transfer(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "transfer" })))
}


pub async fn scrap(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<InventoryResponse>>> {
    Ok(Json(ApiResponse::ok(InventoryResponse { module: "inventory", status: "scrap" })))
}

