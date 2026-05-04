use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/purchase-orders", get(handlers::purchase_orders))
        .route("/receipts", post(handlers::receipts))
}
