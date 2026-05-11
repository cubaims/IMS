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
        .route("/runs/{run_id}", get(handlers::get_run))
        .route("/suggestions", get(handlers::suggestions))
        .route("/suggestions/export", get(handlers::suggestions_export))
        .route(
            "/suggestions/{suggestion_id}",
            get(handlers::get_suggestion),
        )
        .route(
            "/suggestions/{suggestion_id}/confirm",
            post(handlers::confirm_suggestion),
        )
        .route(
            "/suggestions/{suggestion_id}/cancel",
            post(handlers::cancel_suggestion),
        )
}
