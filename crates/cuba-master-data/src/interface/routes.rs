use axum::{routing::{get, post}, Router};
use cuba_shared::AppState;
use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/materials", get(handlers::materials))
        .route("/bins", get(handlers::bins))
        .route("/suppliers", get(handlers::suppliers))
        .route("/customers", get(handlers::customers))
        .route("/boms", get(handlers::boms))
        .route("/variants", get(handlers::variants))
}
