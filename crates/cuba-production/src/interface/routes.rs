use super::handlers;
use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/production-orders", get(handlers::production_orders))
        .route("/complete", post(handlers::complete))
        .route("/variance", get(handlers::variance))
        .route("/bom-explosion", get(handlers::bom_explosion))
}
