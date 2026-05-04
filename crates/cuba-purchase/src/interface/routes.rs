use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

use crate::interface::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::list_purchase_orders).post(handlers::create_purchase_order),
        )
        .route("/{po_id}", get(handlers::get_purchase_order))
        .route("/{po_id}/receipt", post(handlers::post_purchase_receipt))
        .route("/{po_id}/close", post(handlers::close_purchase_order))
}
