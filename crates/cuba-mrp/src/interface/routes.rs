use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/run", post(handlers::run))
        .route("/runs", get(handlers::runs))
        .route("/suggestions", get(handlers::suggestions))
}
