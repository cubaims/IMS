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
        .route("/current-stock/export", get(handlers::current_stock_export))
        .route("/inventory-value", get(handlers::inventory_value))
        .route(
            "/inventory-value/export",
            get(handlers::inventory_value_export),
        )
        .route("/quality-status", get(handlers::quality_status))
        .route("/mrp-shortage", get(handlers::mrp_shortage))
        .route("/mrp-shortage/export", get(handlers::mrp_shortage_export))
        .route("/low-stock-alert", get(handlers::low_stock_alert))
        .route(
            "/low-stock-alert/export",
            get(handlers::low_stock_alert_export),
        )
        .route("/stock-by-zone", get(handlers::stock_by_zone))
        .route("/bin-stock-summary", get(handlers::bin_stock_summary))
        .route("/batch-stock-summary", get(handlers::batch_stock_summary))
        .route(
            "/batch-stock-summary/export",
            get(handlers::batch_stock_summary_export),
        )
        .route("/data-consistency", get(handlers::data_consistency))
        .route(
            "/data-consistency/export",
            get(handlers::data_consistency_export),
        )
        .route("/refresh", post(handlers::refresh))
}
