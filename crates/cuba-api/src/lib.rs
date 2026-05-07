//! cuba-api: HTTP 装配层
//!
//! 只负责：路由聚合、中间件挂载、AppState 注入。
//! 所有业务逻辑在各 cuba-{module} crate 内。

pub mod middleware;
pub mod routes;

use axum::Router;
use cuba_shared::AppState;
use std::time::Duration;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

/// 构建主路由
pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .merge(routes::health::router())
        // ==================== auth 模块 ====================
        .nest("/api/auth", {
            let auth_public = cuba_auth::interface::routes::public_routes();
            let auth_protected = cuba_auth::interface::routes::protected_routes().layer(
                axum::middleware::from_fn_with_state(state.clone(), middleware::auth_middleware),
            );

            auth_public.merge(auth_protected)
        })
        // ==================== master-data 模块（认证 + 权限控制） ====================
        .nest("/api/master-data", {
            cuba_master_data::interface::routes::routes()
                // 所有 master-data 接口都需要登录
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
                // 读权限（查询接口）
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("master-data:read", req, next)
                }))
                // 写权限（创建、修改、启用、停用等）
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("master-data:write", req, next)
                }))
        })
        // ==================== inventory 模块（必须登录 + 权限控制） ====================
        .nest("/api/inventory", {
            cuba_inventory::interface::routes::routes()
                // 所有 inventory 接口都需要登录
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::middleware::auth_middleware,
                ))
                // 读权限（查询库存、流水、批次、历史等）
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("inventory:read", req, next)
                }))
                // 写权限（过账、转储、盘点等）
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("inventory:post", req, next)
                }))
        })
        // ==================== purchase 模块 ====================
        .nest("/api/purchase-orders", {
            cuba_purchase::interface::routes::routes()
                // 必须登录
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::middleware::auth_middleware,
                ))
                // 读权限
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("purchase:read", req, next)
                }))
                // 写权限（创建、收货、关闭等）
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("purchase:write", req, next)
                }))
        })
        .nest("/api/sales-orders", cuba_sales::interface::routes::routes())
        .nest(
            "/api/production",
            cuba_production::interface::routes::production_routes(),
        )
        .nest(
            "/api/production-orders",
            cuba_production::interface::routes::production_order_routes(),
        )
        .nest("/api/quality", cuba_quality::interface::routes::routes())
        // ==================== MRP 模块（认证 + 按路由权限控制） ====================
        .nest("/api/mrp", {
            let public_routes = Router::new()
                .route("/health", axum::routing::get(cuba_mrp::interface::handlers::health));

            let run_routes = Router::new()
                .route("/run", axum::routing::post(cuba_mrp::interface::handlers::run))
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("mrp:run", req, next)
                }));

            let run_read_routes = Router::new()
                .route("/runs", axum::routing::get(cuba_mrp::interface::handlers::runs))
                .route(
                    "/runs/{run_id}",
                    axum::routing::get(cuba_mrp::interface::handlers::get_run),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("mrp:read", req, next)
                }));

            let suggestion_read_routes = Router::new()
                .route(
                    "/suggestions",
                    axum::routing::get(cuba_mrp::interface::handlers::suggestions),
                )
                .route(
                    "/suggestions/{suggestion_id}",
                    axum::routing::get(cuba_mrp::interface::handlers::get_suggestion),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("mrp:suggestion-read", req, next)
                }));

            let suggestion_write_routes = Router::new()
                .route(
                    "/suggestions/{suggestion_id}/confirm",
                    axum::routing::post(cuba_mrp::interface::handlers::confirm_suggestion),
                )
                .route(
                    "/suggestions/{suggestion_id}/cancel",
                    axum::routing::post(cuba_mrp::interface::handlers::cancel_suggestion),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("mrp:suggestion-confirm", req, next)
                }));

            let protected_routes = Router::new()
                .merge(run_routes)
                .merge(run_read_routes)
                .merge(suggestion_read_routes)
                .merge(suggestion_write_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::middleware::auth_middleware,
                ));

            public_routes.merge(protected_routes)
        })
        // ==================== reporting 模块（临时保护：认证 + report:read） ====================
        //
        // TODO:
        // 当前先保护整个 /api/reports，避免报表接口裸奔。
        // 后续应在 cuba-reporting routes 明确后拆分：
        // - 查询接口：report:read
        // - 刷新接口：report:refresh
        // - 导出接口：report:export
        .nest("/api/reports", {
            let public_routes = Router::new()
                .route("/health", axum::routing::get(cuba_reporting::interface::handlers::health));

            let read_routes = Router::new()
                .route(
                    "/current-stock",
                    axum::routing::get(cuba_reporting::interface::handlers::current_stock),
                )
                .route(
                    "/inventory-value",
                    axum::routing::get(cuba_reporting::interface::handlers::inventory_value),
                )
                .route(
                    "/quality-status",
                    axum::routing::get(cuba_reporting::interface::handlers::quality_status),
                )
                .route(
                    "/mrp-shortage",
                    axum::routing::get(cuba_reporting::interface::handlers::mrp_shortage),
                )
                .route(
                    "/low-stock-alert",
                    axum::routing::get(cuba_reporting::interface::handlers::low_stock_alert),
                )
                .route(
                    "/stock-by-zone",
                    axum::routing::get(cuba_reporting::interface::handlers::stock_by_zone),
                )
                .route(
                    "/bin-stock-summary",
                    axum::routing::get(cuba_reporting::interface::handlers::bin_stock_summary),
                )
                .route(
                    "/batch-stock-summary",
                    axum::routing::get(cuba_reporting::interface::handlers::batch_stock_summary),
                )
                .route(
                    "/data-consistency",
                    axum::routing::get(cuba_reporting::interface::handlers::data_consistency),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("report:read", req, next)
                }));

            let refresh_routes = Router::new()
                .route(
                    "/refresh",
                    axum::routing::post(cuba_reporting::interface::handlers::refresh),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("report:refresh", req, next)
                }));

            let protected_routes = Router::new()
                .merge(read_routes)
                .merge(refresh_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::middleware::auth_middleware,
                ));

            public_routes.merge(protected_routes)
        })
        .with_state(state);

    let request_id_header = axum::http::HeaderName::from_static("x-request-id");

    api.layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(build_cors())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(axum::middleware::from_fn(
            middleware::trace::trace_id_middleware,
        ))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(CatchPanicLayer::new())
}

fn build_cors() -> CorsLayer {
    use axum::http::{HeaderValue, Method};

    let allowed = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".into());

    let origins: Vec<HeaderValue> = allowed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| HeaderValue::from_str(s).ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    if origins.is_empty() {
        cors.allow_origin(AllowOrigin::predicate(|_, _| true))
    } else {
        cors.allow_origin(origins)
    }
}
