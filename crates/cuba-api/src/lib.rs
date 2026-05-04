use axum::{routing::get, Router};
use cuba_shared::{ApiResponse, AppResult, AppState};
use serde::Serialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Serialize)]
struct VersionInfo {
    version: &'static str,
    rust_version: &'static str,
    build_date: &'static str,
}

async fn health() -> AppResult<axum::Json<ApiResponse<&'static str>>> {
    Ok(axum::Json(ApiResponse::ok("ims workspace api ready")))
}

async fn version() -> AppResult<axum::Json<ApiResponse<VersionInfo>>> {
    let info = VersionInfo {
        version: env!("CARGO_PKG_VERSION"),
        rust_version: "1.95",
        build_date: option_env!("BUILD_DATE").unwrap_or("unknown"),
    };
    Ok(axum::Json(ApiResponse::ok(info)))
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/version", get(version))
        .nest("/api/auth", cuba_auth::interface::routes::routes())
        .nest("/api/master-data", cuba_master_data::interface::routes::routes())
        .nest("/api/inventory", cuba_inventory::interface::routes::routes())
        .nest("/api/purchase", cuba_purchase::interface::routes::routes())
        .nest("/api/sales", cuba_sales::interface::routes::routes())
        .nest("/api/production", cuba_production::interface::routes::routes())
        .nest("/api/quality", cuba_quality::interface::routes::routes())
        .nest("/api/mrp", cuba_mrp::interface::routes::routes())
        .nest("/api/reports", cuba_reporting::interface::routes::routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
