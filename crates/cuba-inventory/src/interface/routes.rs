use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/transactions", get(handlers::transactions))
        .route("/current-stock", get(handlers::current_stock))
        .route("/bin-stock", get(handlers::bin_stock))
        .route("/batch-stock", get(handlers::batch_stock))
        .route("/transfer", post(handlers::transfer))
        .route("/scrap", post(handlers::scrap))
}
