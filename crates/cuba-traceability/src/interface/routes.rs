use axum::{Router, routing::get};
use cuba_shared::AppState;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/batches/{batch_number}", get(handlers::trace_batch))
        .route("/serials/{serial_number}", get(handlers::trace_serial))
}
