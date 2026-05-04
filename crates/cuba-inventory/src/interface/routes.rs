use axum::{
    routing::{get, post},
    Router,
};

use cuba_shared::AppState;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/post", post(handlers::post_inventory))
        .route("/transfer", post(handlers::transfer_inventory))
        .route("/pick-batch-fefo", post(handlers::pick_batch_fefo))
        .route("/current", get(handlers::list_current_stock))
        .route("/bin-stock", get(handlers::list_bin_stock))
        .route("/transactions", get(handlers::list_transactions))
        .route(
            "/transactions/{transaction_id}",
            get(handlers::get_transaction),
        )
        .route("/batches", get(handlers::list_batches))
        .route("/batches/{batch_number}", get(handlers::get_batch))
        .route(
            "/batches/{batch_number}/history",
            get(handlers::list_batch_history),
        )
        .route("/map-history", get(handlers::list_map_history))
        .route(
            "/materials/{material_id}/map-history",
            get(handlers::list_material_map_history),
        )
}