use axum::{
    Router,
    routing::{get, patch, post},
};
use cuba_shared::AppState;

use super::handlers;

pub fn production_routes() -> Router<AppState> {
    Router::new()
        .route("/bom-explosion", post(handlers::preview_bom_explosion))
        .route("/variances", get(handlers::list_production_variances))
        .route(
            "/batches/{batch_number}/components",
            get(handlers::get_finished_batch_components),
        )
        .route(
            "/batches/{batch_number}/where-used",
            get(handlers::get_component_batch_where_used),
        )
}

pub fn production_order_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_production_orders))
        .route("/", post(handlers::create_production_order))
        .route("/{order_id}", get(handlers::get_production_order))
        .route("/{order_id}", patch(handlers::update_production_order))
        .route(
            "/{order_id}/components",
            get(handlers::get_production_order_components),
        )
        .route(
            "/{order_id}/release",
            post(handlers::release_production_order),
        )
        .route(
            "/{order_id}/cancel",
            post(handlers::cancel_production_order),
        )
        .route("/{order_id}/close", post(handlers::close_production_order))
        .route(
            "/{order_id}/complete",
            post(handlers::complete_production_order),
        )
        .route(
            "/{order_id}/genealogy",
            get(handlers::get_production_genealogy),
        )
        .route(
            "/{order_id}/variance",
            get(handlers::get_production_variance),
        )
}
