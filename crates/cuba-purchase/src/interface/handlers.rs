use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::PurchaseResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<PurchaseResponse>>> {
    Ok(Json(ApiResponse::ok(PurchaseResponse { module: "purchase", status: "ready" })))
}

pub async fn purchase_orders(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<PurchaseResponse>>> {
    Ok(Json(ApiResponse::ok(PurchaseResponse { module: "purchase", status: "purchase-orders" })))
}


pub async fn receipts(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<PurchaseResponse>>> {
    Ok(Json(ApiResponse::ok(PurchaseResponse { module: "purchase", status: "receipts" })))
}

