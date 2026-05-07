use axum::{Json, Router, routing::get};
use cuba_shared::{ApiResponse, AppResult, AppState};
use serde::Serialize;

#[derive(Serialize)]
pub struct VersionInfo {
    pub version: &'static str,
    pub rust_version: &'static str,
    pub build_date: &'static str,
    pub git_sha: &'static str,
}

/// 健康/版本路由。挂在主路由根上即可:
/// - GET /health
/// - GET /api/version
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/version", get(version))
}

async fn health() -> AppResult<Json<ApiResponse<&'static str>>> {
    Ok(Json(ApiResponse::ok("ims workspace api ready")))
}

async fn version() -> AppResult<Json<ApiResponse<VersionInfo>>> {
    Ok(Json(ApiResponse::ok(VersionInfo {
        version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
        build_date: option_env!("BUILD_DATE").unwrap_or("unknown"),
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown"),
    })))
}
