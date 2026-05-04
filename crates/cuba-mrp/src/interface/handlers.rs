use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::MrpResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MrpResponse>>> {
    Ok(Json(ApiResponse::ok(MrpResponse { module: "mrp", status: "ready" })))
}

pub async fn run(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MrpResponse>>> {
    Ok(Json(ApiResponse::ok(MrpResponse { module: "mrp", status: "run" })))
}


pub async fn runs(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MrpResponse>>> {
    Ok(Json(ApiResponse::ok(MrpResponse { module: "mrp", status: "runs" })))
}


pub async fn suggestions(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<MrpResponse>>> {
    Ok(Json(ApiResponse::ok(MrpResponse { module: "mrp", status: "suggestions" })))
}

