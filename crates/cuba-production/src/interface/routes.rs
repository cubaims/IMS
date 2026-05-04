use axum::{
    routing::{get, post},
    Router,
};

use cuba_shared::AppState;

use crate::interface::handlers;

/// 挂载到：/api/production
///
/// 示例：
/// POST /api/production/bom-explosion
/// GET  /api/production/variances
/// GET  /api/production/batches/{batch_number}/components
/// GET  /api/production/batches/{batch_number}/where-used
pub fn production_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bom-explosion",
            post(handlers::preview_bom_explosion),
        )
        .route(
            "/variances",
            get(handlers::list_production_variances),
        )
        .route(
            "/batches/{batch_number}/components",
            get(handlers::get_components_by_finished_batch),
        )
        .route(
            "/batches/{batch_number}/where-used",
            get(handlers::get_where_used_by_component_batch),
        )
}

/// 挂载到：/api/production-orders
///
/// 示例：
/// GET  /api/production-orders
/// POST /api/production-orders
/// GET  /api/production-orders/{order_id}
/// POST /api/production-orders/{order_id}/release
/// POST /api/production-orders/{order_id}/complete
/// GET  /api/production-orders/{order_id}/genealogy
/// GET  /api/production-orders/{order_id}/variance
pub fn production_order_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::list_production_orders)
                .post(handlers::create_production_order),
        )
        .route(
            "/{order_id}",
            get(handlers::get_production_order),
        )
        .route(
            "/{order_id}/release",
            post(handlers::release_production_order),
        )
        .route(
            "/{order_id}/cancel",
            post(handlers::cancel_production_order),
        )
        .route(
            "/{order_id}/close",
            post(handlers::close_production_order),
        )
        .route(
            "/{order_id}/complete",
            post(handlers::complete_production_order),
        )
        .route(
            "/{order_id}/components",
            get(handlers::get_order_components),
        )
        .route(
            "/{order_id}/genealogy",
            get(handlers::get_order_genealogy),
        )
        .route(
            "/{order_id}/variance",
            get(handlers::get_order_variance),
        )
}

/// 兼容旧挂载方式。
/// 如果 cuba-api 仍然使用 `.nest("/api/production", cuba_production::interface::routes::routes())`，
/// 这里默认返回 production_routes。
pub fn routes() -> Router<AppState> {
    production_routes()
}