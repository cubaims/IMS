use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/production-orders", get(handlers::production_orders))
        .route("/complete", post(handlers::complete))
        .route("/variance", get(handlers::variance))
        .route("/bom-explosion", get(handlers::bom_explosion))
}
