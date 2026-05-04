use super::handlers;
use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/run", post(handlers::run))
        .route("/runs", get(handlers::runs))
        .route("/suggestions", get(handlers::suggestions))
}
