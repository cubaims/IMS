use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/sales-orders", get(handlers::sales_orders))
        .route("/shipments", post(handlers::shipments))
}
