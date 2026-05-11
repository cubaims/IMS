use crate::interface::handlers::{
    add_inspection_result_handler, batch_add_inspection_results_handler,
    create_inspection_lot_handler, freeze_batch_handler, get_batch_quality_status_handler,
    get_inspection_lot_handler, get_quality_notification_handler,
    list_batch_quality_history_handler, list_inspection_lots_handler,
    list_inspection_results_handler, list_quality_notifications_handler,
    make_inspection_decision_handler, scrap_batch_handler, unfreeze_batch_handler,
};
use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

/// 构建质量模块路由。
///
/// 由 cuba-api 统一挂载：
///
/// /api/quality/*
pub fn routes() -> Router<AppState> {
    Router::new()
        // 检验批
        .route(
            "/inspection-lots",
            post(create_inspection_lot_handler).get(list_inspection_lots_handler),
        )
        .route("/inspection-lots/{lot_id}", get(get_inspection_lot_handler))
        // 检验结果
        .route(
            "/inspection-lots/{lot_id}/results",
            post(add_inspection_result_handler).get(list_inspection_results_handler),
        )
        .route(
            "/inspection-lots/{lot_id}/results/batch",
            post(batch_add_inspection_results_handler),
        )
        // 质量判定
        .route(
            "/inspection-lots/{lot_id}/decision",
            post(make_inspection_decision_handler),
        )
        // 质量通知
        .route("/notifications", get(list_quality_notifications_handler))
        .route(
            "/notifications/{notification_id}",
            get(get_quality_notification_handler),
        )
        // 批次质量操作
        .route("/batches/{batch_number}/freeze", post(freeze_batch_handler))
        .route(
            "/batches/{batch_number}/unfreeze",
            post(unfreeze_batch_handler),
        )
        .route("/batches/{batch_number}/scrap", post(scrap_batch_handler))
        .route(
            "/batches/{batch_number}/status",
            get(get_batch_quality_status_handler),
        )
        .route(
            "/batches/{batch_number}/history",
            get(list_batch_quality_history_handler),
        )
}
