use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::ReportingResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "ready" })))
}

pub async fn current_stock(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "current-stock" })))
}


pub async fn inventory_value(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "inventory-value" })))
}


pub async fn quality_status(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "quality-status" })))
}


pub async fn mrp_shortage(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "mrp-shortage" })))
}


pub async fn low_stock_alert(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "low-stock-alert" })))
}


pub async fn data_consistency(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "data-consistency" })))
}


pub async fn refresh(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<ReportingResponse>>> {
    Ok(Json(ApiResponse::ok(ReportingResponse { module: "reporting", status: "refresh" })))
}

