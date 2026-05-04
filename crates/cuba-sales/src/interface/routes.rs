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
            get(handlers::list_sales_orders).post(handlers::create_sales_order),
        )
        .route("/{so_id}", get(handlers::get_sales_order))
        .route("/{so_id}/shipment", post(handlers::post_sales_shipment))
        .route(
            "/{so_id}/pick-preview",
            post(handlers::preview_sales_fefo_pick),
        )
        .route("/{so_id}/close", post(handlers::close_sales_order))
}
