use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/login", post(handlers::login))
        .route("/me", get(handlers::me))
        .route("/roles", get(handlers::roles))
        .route("/permissions", get(handlers::permissions))
}
