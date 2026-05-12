use axum::{
    Router,
    routing::{get, patch, post},
};

use cuba_shared::AppState;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        // 库存核心功能
        .route("/post", post(handlers::post_inventory))
        .route("/transfer", post(handlers::transfer_inventory))
        .route("/pick-batch-fefo", post(handlers::pick_batch_fefo))
        .route("/current", get(handlers::list_current_stock))
        .route(
            "/by-material/{material_id}",
            get(handlers::list_current_stock_by_material),
        )
        .route(
            "/by-bin/{bin_code}",
            get(handlers::list_current_stock_by_bin),
        )
        .route(
            "/by-batch/{batch_number}",
            get(handlers::list_current_stock_by_batch),
        )
        .route("/bin-stock", get(handlers::list_bin_stock))
        .route("/stock-by-zone", get(handlers::stock_by_zone))
        .route("/bin-summary", get(handlers::bin_summary))
        .route("/batch-summary", get(handlers::batch_summary))
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
        // ==================== 盘点模块 ====================
        .route("/counts", post(handlers::create_inventory_count))
        .route("/counts", get(handlers::list_inventory_counts))
        .route("/counts/{count_doc_id}", get(handlers::get_inventory_count))
        .route(
            "/counts/{count_doc_id}/differences",
            get(handlers::list_inventory_count_differences),
        )
        .route(
            "/counts/{count_doc_id}/generate-lines",
            post(handlers::generate_inventory_count_lines),
        )
        .route(
            "/counts/{count_doc_id}/lines/{line_no}",
            patch(handlers::update_inventory_count_line),
        )
        .route(
            "/counts/{count_doc_id}/lines/batch",
            patch(handlers::batch_update_inventory_count_lines),
        )
        .route(
            "/counts/{count_doc_id}/lines",
            patch(handlers::batch_update_inventory_count_lines),
        )
        .route(
            "/counts/{count_doc_id}/submit",
            post(handlers::submit_inventory_count),
        )
        .route(
            "/counts/{count_doc_id}/approve",
            post(handlers::approve_inventory_count),
        )
        .route(
            "/counts/{count_doc_id}/post",
            post(handlers::post_inventory_count),
        )
        .route(
            "/counts/{count_doc_id}/close",
            post(handlers::close_inventory_count),
        )
        .route(
            "/counts/{count_doc_id}/cancel",
            post(handlers::cancel_inventory_count),
        )
}
