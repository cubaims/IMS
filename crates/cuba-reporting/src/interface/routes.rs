use super::handlers;
use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/current-stock", get(handlers::current_stock))
        .route("/inventory-value", get(handlers::inventory_value))
        .route("/quality-status", get(handlers::quality_status))
        .route("/mrp-shortage", get(handlers::mrp_shortage))
        .route("/low-stock-alert", get(handlers::low_stock_alert))
        .route("/data-consistency", get(handlers::data_consistency))
        .route("/refresh", post(handlers::refresh))
}
