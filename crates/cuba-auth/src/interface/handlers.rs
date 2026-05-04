use axum::{extract::State, Json};
use cuba_shared::{ApiResponse, AppResult, AppState};
use super::dto::AuthResponse;

pub async fn health(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse { module: "auth", status: "ready" })))
}

pub async fn login(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse { module: "auth", status: "login" })))
}


pub async fn me(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse { module: "auth", status: "me" })))
}


pub async fn roles(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse { module: "auth", status: "roles" })))
}


pub async fn permissions(State(_state): State<AppState>) -> AppResult<Json<ApiResponse<AuthResponse>>> {
    Ok(Json(ApiResponse::ok(AuthResponse { module: "auth", status: "permissions" })))
}

