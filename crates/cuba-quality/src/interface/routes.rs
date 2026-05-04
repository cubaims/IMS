use super::handlers;
use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/inspection-lots", get(handlers::inspection_lots))
        .route("/inspection-results", get(handlers::inspection_results))
        .route("/notifications", get(handlers::notifications))
        .route("/decisions", post(handlers::decisions))
}
