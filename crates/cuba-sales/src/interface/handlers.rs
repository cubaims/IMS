use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::SalesResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<SalesResponse>>> {
    Ok(Json(ApiResponse::ok(SalesResponse { module: "sales", status: "ready" })))
}

pub async fn sales_orders(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<SalesResponse>>> {
    Ok(Json(ApiResponse::ok(SalesResponse { module: "sales", status: "sales-orders" })))
}


pub async fn shipments(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<SalesResponse>>> {
    Ok(Json(ApiResponse::ok(SalesResponse { module: "sales", status: "shipments" })))
}

