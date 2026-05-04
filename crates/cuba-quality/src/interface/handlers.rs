use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::QualityResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<QualityResponse>>> {
    Ok(Json(ApiResponse::ok(QualityResponse { module: "quality", status: "ready" })))
}

pub async fn inspection_lots(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<QualityResponse>>> {
    Ok(Json(ApiResponse::ok(QualityResponse { module: "quality", status: "inspection-lots" })))
}


pub async fn inspection_results(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<QualityResponse>>> {
    Ok(Json(ApiResponse::ok(QualityResponse { module: "quality", status: "inspection-results" })))
}


pub async fn notifications(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<QualityResponse>>> {
    Ok(Json(ApiResponse::ok(QualityResponse { module: "quality", status: "notifications" })))
}


pub async fn decisions(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<QualityResponse>>> {
    Ok(Json(ApiResponse::ok(QualityResponse { module: "quality", status: "decisions" })))
}

