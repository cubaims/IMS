//! cuba-api: HTTP 装配层
//!
//! 只负责：路由聚合、中间件挂载、AppState 注入。
//! 所有业务逻辑在各 cuba-{module} crate 内。

pub mod middleware;
pub mod routes;

use axum::{Router, http::header, response::IntoResponse};
use cuba_shared::AppState;
use std::time::Duration;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

const MASTER_DATA_OPENAPI_JSON: &str =
    include_str!("../../../docs/openapi/master-data.phase3.openapi.json");
const INVENTORY_CORE_OPENAPI_JSON: &str =
    include_str!("../../../docs/openapi/inventory-core.phase4.openapi.json");
const INVENTORY_COUNT_OPENAPI_JSON: &str =
    include_str!("../../../docs/openapi/inventory-count.phase7.openapi.json");
const ORDER_PHASE5_OPENAPI_JSON: &str =
    include_str!("../../../docs/openapi/order-phase5.openapi.json");
const PRODUCTION_PHASE6_OPENAPI_JSON: &str =
    include_str!("../../../docs/openapi/production.phase6.openapi.json");
const MRP_REPORTING_OPENAPI_JSON: &str =
    include_str!("../../../docs/openapi/mrp-reporting.phase9.openapi.json");

/// 构建主路由
pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .merge(routes::health::router())
        .merge(
            routes::system::router()
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                )),
        )
        .route(
            "/api/openapi/master-data.json",
            axum::routing::get(master_data_openapi),
        )
        .route(
            "/api/openapi/inventory-core.json",
            axum::routing::get(inventory_core_openapi),
        )
        .route(
            "/api/openapi/inventory-count.json",
            axum::routing::get(inventory_count_openapi),
        )
        .route(
            "/api/openapi/orders-phase5.json",
            axum::routing::get(order_phase5_openapi),
        )
        .route(
            "/api/openapi/production-phase6.json",
            axum::routing::get(production_phase6_openapi),
        )
        .route(
            "/api/openapi/mrp-reporting.json",
            axum::routing::get(mrp_reporting_openapi),
        )
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
            let read_routes = cuba_master_data::interface::routes::read_routes().layer(
                axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("master-data:read", req, next)
                }),
            );

            let write_routes = cuba_master_data::interface::routes::write_routes().layer(
                axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("master-data:write", req, next)
                }),
            );

            read_routes
                .merge(write_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
        })
        // ==================== inventory 模块 ====================
        .nest("/api/inventory", {
            let read_routes = Router::new()
                .route(
                    "/current",
                    axum::routing::get(cuba_inventory::interface::handlers::list_current_stock),
                )
                .route(
                    "/by-material/{material_id}",
                    axum::routing::get(
                        cuba_inventory::interface::handlers::list_current_stock_by_material,
                    ),
                )
                .route(
                    "/by-bin/{bin_code}",
                    axum::routing::get(cuba_inventory::interface::handlers::list_current_stock_by_bin),
                )
                .route(
                    "/by-batch/{batch_number}",
                    axum::routing::get(
                        cuba_inventory::interface::handlers::list_current_stock_by_batch,
                    ),
                )
                .route(
                    "/bin-stock",
                    axum::routing::get(cuba_inventory::interface::handlers::list_bin_stock),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory:read", req, next)
                }));

            let history_routes = Router::new()
                .route(
                    "/transactions",
                    axum::routing::get(cuba_inventory::interface::handlers::list_transactions),
                )
                .route(
                    "/transactions/{transaction_id}",
                    axum::routing::get(cuba_inventory::interface::handlers::get_transaction),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory:history", req, next)
                }));

            let batch_read_routes = Router::new()
                .route(
                    "/batches",
                    axum::routing::get(cuba_inventory::interface::handlers::list_batches),
                )
                .route(
                    "/batches/{batch_number}",
                    axum::routing::get(cuba_inventory::interface::handlers::get_batch),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("batch:read", req, next)
                }));

            let batch_history_routes = Router::new()
                .route(
                    "/batches/{batch_number}/history",
                    axum::routing::get(cuba_inventory::interface::handlers::list_batch_history),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("batch:history", req, next)
                }));

            let map_history_routes = Router::new()
                .route(
                    "/map-history",
                    axum::routing::get(cuba_inventory::interface::handlers::list_map_history),
                )
                .route(
                    "/materials/{material_id}/map-history",
                    axum::routing::get(
                        cuba_inventory::interface::handlers::list_material_map_history,
                    ),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("cost:map-read", req, next)
                }));

            let report_routes = Router::new()
                .route(
                    "/stock-by-zone",
                    axum::routing::get(cuba_inventory::interface::handlers::stock_by_zone),
                )
                .route(
                    "/bin-summary",
                    axum::routing::get(cuba_inventory::interface::handlers::bin_summary),
                )
                .route(
                    "/batch-summary",
                    axum::routing::get(cuba_inventory::interface::handlers::batch_summary),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("report:read", req, next)
                }));

            let write_routes = Router::new()
                .route(
                    "/post",
                    axum::routing::post(cuba_inventory::interface::handlers::post_inventory),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory:post", req, next)
                }));

            let transfer_routes = Router::new()
                .route(
                    "/transfer",
                    axum::routing::post(cuba_inventory::interface::handlers::transfer_inventory),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory:transfer", req, next)
                }));

            let fefo_routes = Router::new()
                .route(
                    "/pick-batch-fefo",
                    axum::routing::post(cuba_inventory::interface::handlers::pick_batch_fefo),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("batch:read", req, next)
                }));

            let count_read_routes = Router::new()
                .route(
                    "/counts",
                    axum::routing::get(cuba_inventory::interface::handlers::list_inventory_counts),
                )
                .route(
                    "/counts/{count_doc_id}",
                    axum::routing::get(cuba_inventory::interface::handlers::get_inventory_count),
                )
                .route(
                    "/counts/{count_doc_id}/differences",
                    axum::routing::get(
                        cuba_inventory::interface::handlers::list_inventory_count_differences,
                    ),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory-count:read", req, next)
                }));

            let count_write_routes = Router::new()
                .route(
                    "/counts",
                    axum::routing::post(cuba_inventory::interface::handlers::create_inventory_count),
                )
                .route(
                    "/counts/{count_doc_id}/generate-lines",
                    axum::routing::post(
                        cuba_inventory::interface::handlers::generate_inventory_count_lines,
                    ),
                )
                .route(
                    "/counts/{count_doc_id}/lines/{line_no}",
                    axum::routing::patch(
                        cuba_inventory::interface::handlers::update_inventory_count_line,
                    ),
                )
                .route(
                    "/counts/{count_doc_id}/lines/batch",
                    axum::routing::patch(
                        cuba_inventory::interface::handlers::batch_update_inventory_count_lines,
                    ),
                )
                .route(
                    "/counts/{count_doc_id}/lines",
                    axum::routing::patch(
                        cuba_inventory::interface::handlers::batch_update_inventory_count_lines,
                    ),
                )
                .route(
                    "/counts/{count_doc_id}/cancel",
                    axum::routing::post(cuba_inventory::interface::handlers::cancel_inventory_count),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory-count:write", req, next)
                }));

            let count_submit_routes = Router::new()
                .route(
                    "/counts/{count_doc_id}/submit",
                    axum::routing::post(cuba_inventory::interface::handlers::submit_inventory_count),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory-count:submit", req, next)
                }));

            let count_approve_routes = Router::new()
                .route(
                    "/counts/{count_doc_id}/approve",
                    axum::routing::post(cuba_inventory::interface::handlers::approve_inventory_count),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory-count:approve", req, next)
                }));

            let count_post_routes = Router::new()
                .route(
                    "/counts/{count_doc_id}/post",
                    axum::routing::post(cuba_inventory::interface::handlers::post_inventory_count),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory-count:post", req, next)
                }));

            let count_close_routes = Router::new()
                .route(
                    "/counts/{count_doc_id}/close",
                    axum::routing::post(cuba_inventory::interface::handlers::close_inventory_count),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("inventory-count:close", req, next)
                }));

            read_routes
                .merge(history_routes)
                .merge(batch_read_routes)
                .merge(batch_history_routes)
                .merge(map_history_routes)
                .merge(report_routes)
                .merge(write_routes)
                .merge(transfer_routes)
                .merge(fefo_routes)
                .merge(count_read_routes)
                .merge(count_write_routes)
                .merge(count_submit_routes)
                .merge(count_approve_routes)
                .merge(count_post_routes)
                .merge(count_close_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
        })
        // ==================== purchase 模块 ====================
        .nest("/api/purchase-orders", {
            let read_routes = Router::new()
                .route(
                    "/",
                    axum::routing::get(cuba_purchase::interface::handlers::list_purchase_orders),
                )
                .route(
                    "/{po_id}",
                    axum::routing::get(cuba_purchase::interface::handlers::get_purchase_order),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("purchase:read", req, next)
                }));

            let write_routes = Router::new()
                .route(
                    "/",
                    axum::routing::post(cuba_purchase::interface::handlers::create_purchase_order),
                )
                .route(
                    "/{po_id}",
                    axum::routing::patch(
                        cuba_purchase::interface::handlers::update_purchase_order,
                    ),
                )
                .route(
                    "/{po_id}/close",
                    axum::routing::post(cuba_purchase::interface::handlers::close_purchase_order),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("purchase:write", req, next)
                }));

            let receipt_routes = Router::new()
                .route(
                    "/{po_id}/receipt",
                    axum::routing::post(cuba_purchase::interface::handlers::post_purchase_receipt),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("purchase:receipt", req, next)
                }));

            read_routes
                .merge(write_routes)
                .merge(receipt_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
        })
        .nest("/api/sales-orders", {
            let read_routes = Router::new()
                .route(
                    "/",
                    axum::routing::get(cuba_sales::interface::handlers::list_sales_orders),
                )
                .route(
                    "/{so_id}",
                    axum::routing::get(cuba_sales::interface::handlers::get_sales_order),
                )
                .route(
                    "/{so_id}/pick-preview",
                    axum::routing::post(cuba_sales::interface::handlers::preview_sales_fefo_pick),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("sales:read", req, next)
                }));

            let write_routes = Router::new()
                .route(
                    "/",
                    axum::routing::post(cuba_sales::interface::handlers::create_sales_order),
                )
                .route(
                    "/{so_id}",
                    axum::routing::patch(cuba_sales::interface::handlers::update_sales_order),
                )
                .route(
                    "/{so_id}/close",
                    axum::routing::post(cuba_sales::interface::handlers::close_sales_order),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("sales:write", req, next)
                }));

            let shipment_routes = Router::new()
                .route(
                    "/{so_id}/shipment",
                    axum::routing::post(cuba_sales::interface::handlers::post_sales_shipment),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("sales:shipment", req, next)
                }));

            read_routes
                .merge(write_routes)
                .merge(shipment_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
        })
        .nest(
            "/api/production",
            Router::new()
                .route(
                    "/bom-explosion",
                    axum::routing::post(cuba_production::interface::handlers::preview_bom_explosion),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("bom:explode", req, next)
                }))
                .merge(
                    Router::new()
                        .route(
                            "/variances",
                            axum::routing::get(
                                cuba_production::interface::handlers::list_production_variances,
                            ),
                        )
                        .layer(axum::middleware::from_fn(|req, next| {
                            middleware::require_permission("production:variance-read", req, next)
                        })),
                )
                .merge(
                    Router::new()
                        .route(
                            "/batches/{batch_number}/components",
                            axum::routing::get(
                                cuba_production::interface::handlers::get_finished_batch_components,
                            ),
                        )
                        .route(
                            "/batches/{batch_number}/where-used",
                            axum::routing::get(
                                cuba_production::interface::handlers::get_component_batch_where_used,
                            ),
                        )
                        .layer(axum::middleware::from_fn(|req, next| {
                            middleware::require_permission("batch:trace", req, next)
                        })),
                )
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                )),
        )
        .nest(
            "/api/production-orders",
            {
                let read_routes = Router::new()
                    .route(
                        "/",
                        axum::routing::get(
                            cuba_production::interface::handlers::list_production_orders,
                        ),
                    )
                    .route(
                        "/{order_id}",
                        axum::routing::get(
                            cuba_production::interface::handlers::get_production_order,
                        ),
                    )
                    .route(
                        "/{order_id}/components",
                        axum::routing::get(
                            cuba_production::interface::handlers::get_production_order_components,
                        ),
                    )
                    .layer(axum::middleware::from_fn(|req, next| {
                        middleware::require_permission("production:read", req, next)
                    }));

                let write_routes = Router::new()
                    .route(
                        "/",
                        axum::routing::post(
                            cuba_production::interface::handlers::create_production_order,
                        ),
                    )
                    .route(
                        "/{order_id}",
                        axum::routing::patch(
                            cuba_production::interface::handlers::update_production_order,
                        ),
                    )
                    .route(
                        "/{order_id}/cancel",
                        axum::routing::post(
                            cuba_production::interface::handlers::cancel_production_order,
                        ),
                    )
                    .route(
                        "/{order_id}/close",
                        axum::routing::post(
                            cuba_production::interface::handlers::close_production_order,
                        ),
                    )
                    .layer(axum::middleware::from_fn(|req, next| {
                        middleware::require_permission("production:write", req, next)
                    }));

                let release_routes = Router::new()
                    .route(
                        "/{order_id}/release",
                        axum::routing::post(
                            cuba_production::interface::handlers::release_production_order,
                        ),
                    )
                    .layer(axum::middleware::from_fn(|req, next| {
                        middleware::require_permission("production:release", req, next)
                    }));

                let complete_routes = Router::new()
                    .route(
                        "/{order_id}/complete",
                        axum::routing::post(
                            cuba_production::interface::handlers::complete_production_order,
                        ),
                    )
                    .layer(axum::middleware::from_fn(|req, next| {
                        middleware::require_permission("production:complete", req, next)
                    }));

                let trace_routes = Router::new()
                    .route(
                        "/{order_id}/genealogy",
                        axum::routing::get(
                            cuba_production::interface::handlers::get_production_genealogy,
                        ),
                    )
                    .layer(axum::middleware::from_fn(|req, next| {
                        middleware::require_permission("batch:trace", req, next)
                    }));

                let variance_routes = Router::new()
                    .route(
                        "/{order_id}/variance",
                        axum::routing::get(
                            cuba_production::interface::handlers::get_production_variance,
                        ),
                    )
                    .layer(axum::middleware::from_fn(|req, next| {
                        middleware::require_permission("production:variance-read", req, next)
                    }));

                read_routes
                    .merge(write_routes)
                    .merge(release_routes)
                    .merge(complete_routes)
                    .merge(trace_routes)
                    .merge(variance_routes)
                    .layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        middleware::auth_middleware,
                    ))
            },
        )
        .nest("/api/quality", {
            let read_routes = Router::new()
                .route(
                    "/inspection-lots",
                    axum::routing::get(cuba_quality::interface::handlers::list_inspection_lots_handler),
                )
                .route(
                    "/inspection-lots/{lot_id}",
                    axum::routing::get(cuba_quality::interface::handlers::get_inspection_lot_handler),
                )
                .route(
                    "/inspection-lots/{lot_id}/results",
                    axum::routing::get(cuba_quality::interface::handlers::list_inspection_results_handler),
                )
                .route(
                    "/notifications",
                    axum::routing::get(cuba_quality::interface::handlers::list_quality_notifications_handler),
                )
                .route(
                    "/notifications/{notification_id}",
                    axum::routing::get(cuba_quality::interface::handlers::get_quality_notification_handler),
                )
                .route(
                    "/batches/{batch_number}/status",
                    axum::routing::get(cuba_quality::interface::handlers::get_batch_quality_status_handler),
                )
                .route(
                    "/batches/{batch_number}/history",
                    axum::routing::get(cuba_quality::interface::handlers::list_batch_quality_history_handler),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("quality:read", req, next)
                }));

            let write_routes = Router::new()
                .route(
                    "/inspection-lots",
                    axum::routing::post(cuba_quality::interface::handlers::create_inspection_lot_handler),
                )
                .route(
                    "/inspection-lots/{lot_id}/results",
                    axum::routing::post(cuba_quality::interface::handlers::add_inspection_result_handler),
                )
                .route(
                    "/inspection-lots/{lot_id}/results/batch",
                    axum::routing::post(cuba_quality::interface::handlers::batch_add_inspection_results_handler),
                )
                .route(
                    "/batches/{batch_number}/freeze",
                    axum::routing::post(cuba_quality::interface::handlers::freeze_batch_handler),
                )
                .route(
                    "/batches/{batch_number}/unfreeze",
                    axum::routing::post(cuba_quality::interface::handlers::unfreeze_batch_handler),
                )
                .route(
                    "/batches/{batch_number}/scrap",
                    axum::routing::post(cuba_quality::interface::handlers::scrap_batch_handler),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("quality:write", req, next)
                }));

            let decision_routes = Router::new()
                .route(
                    "/inspection-lots/{lot_id}/decision",
                    axum::routing::post(cuba_quality::interface::handlers::make_inspection_decision_handler),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    middleware::require_permission("quality:decision", req, next)
                }));

            read_routes
                .merge(write_routes)
                .merge(decision_routes)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                ))
        })
        // ==================== MRP 模块（认证 + 按路由权限控制） ====================
        .nest("/api/mrp", {
            let public_routes = Router::new().route(
                "/health",
                axum::routing::get(cuba_mrp::interface::handlers::health),
            );

            let run_routes = Router::new()
                .route(
                    "/run",
                    axum::routing::post(cuba_mrp::interface::handlers::run),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("mrp:run", req, next)
                }));

            let run_read_routes = Router::new()
                .route(
                    "/runs",
                    axum::routing::get(cuba_mrp::interface::handlers::runs),
                )
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
                    "/suggestions/export",
                    axum::routing::get(cuba_mrp::interface::handlers::suggestions_export),
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
        // ==================== traceability 查询编排模块 ====================
        .nest("/api/traceability", {
            let public_routes = Router::new().route(
                "/health",
                axum::routing::get(cuba_traceability::interface::handlers::health),
            );

            let trace_routes = Router::new()
                .route(
                    "/batches/{batch_number}",
                    axum::routing::get(cuba_traceability::interface::handlers::trace_batch),
                )
                .route(
                    "/serials/{serial_number}",
                    axum::routing::get(cuba_traceability::interface::handlers::trace_serial),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("traceability:read", req, next)
                }));

            public_routes.merge(trace_routes.layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::auth_middleware,
            )))
        })
        // ==================== reporting 模块 ====================
        .nest("/api/reports", {
            let public_routes = Router::new().route(
                "/health",
                axum::routing::get(cuba_reporting::interface::handlers::health),
            );

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

            let export_routes = Router::new()
                .route(
                    "/current-stock/export",
                    axum::routing::get(cuba_reporting::interface::handlers::current_stock_export),
                )
                .route(
                    "/inventory-value/export",
                    axum::routing::get(cuba_reporting::interface::handlers::inventory_value_export),
                )
                .route(
                    "/quality-status/export",
                    axum::routing::get(cuba_reporting::interface::handlers::quality_status_export),
                )
                .route(
                    "/mrp-shortage/export",
                    axum::routing::get(cuba_reporting::interface::handlers::mrp_shortage_export),
                )
                .route(
                    "/low-stock-alert/export",
                    axum::routing::get(cuba_reporting::interface::handlers::low_stock_alert_export),
                )
                .route(
                    "/stock-by-zone/export",
                    axum::routing::get(cuba_reporting::interface::handlers::stock_by_zone_export),
                )
                .route(
                    "/bin-stock-summary/export",
                    axum::routing::get(cuba_reporting::interface::handlers::bin_stock_summary_export),
                )
                .route(
                    "/batch-stock-summary/export",
                    axum::routing::get(cuba_reporting::interface::handlers::batch_stock_summary_export),
                )
                .route(
                    "/data-consistency/export",
                    axum::routing::get(cuba_reporting::interface::handlers::data_consistency_export),
                )
                .layer(axum::middleware::from_fn(|req, next| {
                    crate::middleware::require_permission("report:export", req, next)
                }));

            let protected_routes = Router::new()
                .merge(read_routes)
                .merge(refresh_routes)
                .merge(export_routes)
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
        // x-request-id 由 SetRequestIdLayer 生成,PropagateRequestIdLayer 透传到响应头。
        // 之前自己写的 trace_id_middleware 只是把 trace_id 塞 response extensions、没人读,
        // 已删除。
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(CatchPanicLayer::new())
        // 静态文件服务（fallback）
        .fallback_service(ServeDir::new("static"))
}

async fn master_data_openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        MASTER_DATA_OPENAPI_JSON,
    )
}

async fn inventory_core_openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        INVENTORY_CORE_OPENAPI_JSON,
    )
}

async fn inventory_count_openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        INVENTORY_COUNT_OPENAPI_JSON,
    )
}

async fn order_phase5_openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        ORDER_PHASE5_OPENAPI_JSON,
    )
}

async fn production_phase6_openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        PRODUCTION_PHASE6_OPENAPI_JSON,
    )
}

async fn mrp_reporting_openapi() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        MRP_REPORTING_OPENAPI_JSON,
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use cuba_auth::{LoginUseCase, User};
    use sqlx::postgres::PgPoolOptions;
    use time::OffsetDateTime;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_state() -> AppState {
        let db_pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool");

        AppState {
            db_pool,
            jwt_secret: "test-secret".to_string(),
            jwt_issuer: "ims-test".to_string(),
            jwt_expires_seconds: 3600,
            jwt_refresh_expires_seconds: 7200,
        }
    }

    fn issue_token(roles: Vec<&str>, permissions: Vec<&str>) -> String {
        let user = User {
            user_id: Uuid::nil(),
            username: "tester".to_string(),
            password_hash: "not-used".to_string(),
            full_name: Some("Tester".to_string()),
            email: Some("tester@example.com".to_string()),
            role_id: None,
            is_active: true,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        let use_case = LoginUseCase::new("test-secret".to_string(), "ims-test".to_string(), 3600);
        use_case
            .issue_access_token(
                &user,
                &roles
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                &permissions
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            )
            .expect("token")
    }

    fn test_router(state: AppState) -> Router {
        let read_routes = Router::new()
            .route("/read", axum::routing::get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("master-data:read", req, next)
            }));

        let write_routes = Router::new()
            .route("/write", axum::routing::post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("master-data:write", req, next)
            }));

        read_routes
            .merge(write_routes)
            .layer(axum::middleware::from_fn_with_state(
                state,
                middleware::auth_middleware,
            ))
    }

    fn inventory_permission_router(state: AppState) -> Router {
        let read_routes = Router::new()
            .route("/current", axum::routing::get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory:read", req, next)
            }));

        let history_routes = Router::new()
            .route(
                "/transactions",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory:history", req, next)
            }));

        let batch_read_routes = Router::new()
            .route("/batches", axum::routing::get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("batch:read", req, next)
            }));

        let batch_history_routes = Router::new()
            .route(
                "/batches/BATCH-001/history",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("batch:history", req, next)
            }));

        let map_history_routes = Router::new()
            .route(
                "/materials/RM001/map-history",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("cost:map-read", req, next)
            }));

        let report_routes = Router::new()
            .route(
                "/stock-by-zone",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("report:read", req, next)
            }));

        let post_routes = Router::new()
            .route("/post", axum::routing::post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory:post", req, next)
            }));

        let transfer_routes = Router::new()
            .route(
                "/transfer",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory:transfer", req, next)
            }));

        let count_read_routes = Router::new()
            .route("/counts", axum::routing::get(|| async { StatusCode::OK }))
            .route(
                "/counts/CNT-1",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .route(
                "/counts/CNT-1/differences",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory-count:read", req, next)
            }));

        let count_write_routes = Router::new()
            .route("/counts", axum::routing::post(|| async { StatusCode::OK }))
            .route(
                "/counts/CNT-1/generate-lines",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .route(
                "/counts/CNT-1/lines/10",
                axum::routing::patch(|| async { StatusCode::OK }),
            )
            .route(
                "/counts/CNT-1/lines",
                axum::routing::patch(|| async { StatusCode::OK }),
            )
            .route(
                "/counts/CNT-1/cancel",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory-count:write", req, next)
            }));

        let count_submit_routes = Router::new()
            .route(
                "/counts/CNT-1/submit",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory-count:submit", req, next)
            }));

        let count_approve_routes = Router::new()
            .route(
                "/counts/CNT-1/approve",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory-count:approve", req, next)
            }));

        let count_post_routes = Router::new()
            .route(
                "/counts/CNT-1/post",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory-count:post", req, next)
            }));

        let count_close_routes = Router::new()
            .route(
                "/counts/CNT-1/close",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("inventory-count:close", req, next)
            }));

        read_routes
            .merge(history_routes)
            .merge(batch_read_routes)
            .merge(batch_history_routes)
            .merge(map_history_routes)
            .merge(report_routes)
            .merge(post_routes)
            .merge(transfer_routes)
            .merge(count_read_routes)
            .merge(count_write_routes)
            .merge(count_submit_routes)
            .merge(count_approve_routes)
            .merge(count_post_routes)
            .merge(count_close_routes)
            .layer(axum::middleware::from_fn_with_state(
                state,
                middleware::auth_middleware,
            ))
    }

    fn order_permission_router(state: AppState) -> Router {
        let purchase_write_routes = Router::new()
            .route(
                "/purchase-orders",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .route(
                "/purchase-orders/PO-1",
                axum::routing::patch(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("purchase:write", req, next)
            }));

        let purchase_receipt_routes = Router::new()
            .route(
                "/purchase-orders/PO-1/receipt",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("purchase:receipt", req, next)
            }));

        let sales_write_routes = Router::new()
            .route(
                "/sales-orders",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .route(
                "/sales-orders/SO-1",
                axum::routing::patch(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("sales:write", req, next)
            }));

        let sales_shipment_routes = Router::new()
            .route(
                "/sales-orders/SO-1/shipment",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("sales:shipment", req, next)
            }));

        purchase_write_routes
            .merge(purchase_receipt_routes)
            .merge(sales_write_routes)
            .merge(sales_shipment_routes)
            .layer(axum::middleware::from_fn_with_state(
                state,
                middleware::auth_middleware,
            ))
    }

    fn phase9_permission_router(state: AppState) -> Router {
        let mrp_run_routes = Router::new()
            .route("/mrp/run", axum::routing::post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("mrp:run", req, next)
            }));

        let mrp_read_routes = Router::new()
            .route("/mrp/runs", axum::routing::get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("mrp:read", req, next)
            }));

        let suggestion_read_routes = Router::new()
            .route(
                "/mrp/suggestions",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("mrp:suggestion-read", req, next)
            }));

        let suggestion_confirm_routes = Router::new()
            .route(
                "/mrp/suggestions/1/confirm",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("mrp:suggestion-confirm", req, next)
            }));

        let report_read_routes = Router::new()
            .route(
                "/reports/current-stock",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("report:read", req, next)
            }));

        let report_refresh_routes = Router::new()
            .route(
                "/reports/refresh",
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("report:refresh", req, next)
            }));

        let report_export_routes = Router::new()
            .route(
                "/reports/current-stock/export",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn(|req, next| {
                middleware::require_permission("report:export", req, next)
            }));

        mrp_run_routes
            .merge(mrp_read_routes)
            .merge(suggestion_read_routes)
            .merge(suggestion_confirm_routes)
            .merge(report_read_routes)
            .merge(report_refresh_routes)
            .merge(report_export_routes)
            .layer(axum::middleware::from_fn_with_state(
                state,
                middleware::auth_middleware,
            ))
    }

    async fn send(
        app: &Router,
        method: axum::http::Method,
        path: &str,
        token: Option<&str>,
    ) -> StatusCode {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::empty())
                    .expect("request builder should be valid"),
            )
            .await
            .expect("router should respond");
        response.status()
    }

    #[tokio::test]
    async fn openapi_document_contains_master_data_paths() {
        assert!(MASTER_DATA_OPENAPI_JSON.contains("/api/master-data/materials"));
        assert!(
            MASTER_DATA_OPENAPI_JSON
                .contains("/api/master-data/boms/{bom_id}/components/{component_id}")
        );
        assert!(!MASTER_DATA_OPENAPI_JSON.contains("line_no"));
    }

    #[tokio::test]
    async fn openapi_document_contains_phase4_inventory_paths() {
        for path in [
            "/api/inventory/current",
            "/api/inventory/post",
            "/api/inventory/transfer",
            "/api/inventory/transactions/{transaction_id}",
            "/api/inventory/batches/{batch_number}/history",
            "/api/inventory/materials/{material_id}/map-history",
            "/api/inventory/pick-batch-fefo",
        ] {
            assert!(
                INVENTORY_CORE_OPENAPI_JSON.contains(path),
                "missing Phase 4 OpenAPI path: {path}"
            );
        }
    }

    #[tokio::test]
    async fn openapi_document_contains_phase7_inventory_count_paths_and_tags() {
        for path in [
            "/api/inventory/counts",
            "/api/inventory/counts/{count_doc_id}",
            "/api/inventory/counts/{count_doc_id}/differences",
            "/api/inventory/counts/{count_doc_id}/generate-lines",
            "/api/inventory/counts/{count_doc_id}/lines/{line_no}",
            "/api/inventory/counts/{count_doc_id}/lines",
            "/api/inventory/counts/{count_doc_id}/lines/batch",
            "/api/inventory/counts/{count_doc_id}/submit",
            "/api/inventory/counts/{count_doc_id}/approve",
            "/api/inventory/counts/{count_doc_id}/post",
            "/api/inventory/counts/{count_doc_id}/close",
            "/api/inventory/counts/{count_doc_id}/cancel",
        ] {
            assert!(
                INVENTORY_COUNT_OPENAPI_JSON.contains(path),
                "missing Phase 7 OpenAPI path: {path}"
            );
        }

        for tag in [
            "Inventory Count",
            "Inventory Count Lines",
            "Inventory Count Posting",
        ] {
            assert!(
                INVENTORY_COUNT_OPENAPI_JSON.contains(tag),
                "missing Phase 7 OpenAPI tag: {tag}"
            );
        }
    }

    #[tokio::test]
    async fn openapi_document_contains_phase5_order_paths_and_permissions() {
        for path in [
            "/api/purchase-orders",
            "/api/purchase-orders/{po_id}",
            "/api/purchase-orders/{po_id}/receipt",
            "/api/sales-orders",
            "/api/sales-orders/{so_id}",
            "/api/sales-orders/{so_id}/pick-preview",
            "/api/sales-orders/{so_id}/shipment",
        ] {
            assert!(
                ORDER_PHASE5_OPENAPI_JSON.contains(path),
                "missing Phase 5 OpenAPI path: {path}"
            );
        }

        for permission in [
            "purchase:read",
            "purchase:write",
            "purchase:receipt",
            "sales:read",
            "sales:write",
            "sales:shipment",
        ] {
            assert!(
                ORDER_PHASE5_OPENAPI_JSON.contains(permission),
                "missing Phase 5 OpenAPI permission: {permission}"
            );
        }
    }

    #[tokio::test]
    async fn openapi_document_contains_phase6_production_paths_and_permissions() {
        for path in [
            "/api/production/bom-explosion",
            "/api/production/variances",
            "/api/production/batches/{batch_number}/components",
            "/api/production/batches/{batch_number}/where-used",
            "/api/production-orders",
            "/api/production-orders/{order_id}",
            "/api/production-orders/{order_id}/components",
            "/api/production-orders/{order_id}/release",
            "/api/production-orders/{order_id}/complete",
            "/api/production-orders/{order_id}/cancel",
            "/api/production-orders/{order_id}/close",
            "/api/production-orders/{order_id}/genealogy",
            "/api/production-orders/{order_id}/variance",
        ] {
            assert!(
                PRODUCTION_PHASE6_OPENAPI_JSON.contains(path),
                "missing Phase 6 OpenAPI path: {path}"
            );
        }

        for permission in [
            "production:read",
            "production:write",
            "production:release",
            "production:complete",
            "production:variance-read",
            "bom:explode",
            "batch:trace",
        ] {
            assert!(
                PRODUCTION_PHASE6_OPENAPI_JSON.contains(permission),
                "missing Phase 6 OpenAPI permission: {permission}"
            );
        }
    }

    #[tokio::test]
    async fn openapi_document_contains_phase9_paths_and_tags() {
        for path in [
            "/api/mrp/run",
            "/api/mrp/runs",
            "/api/mrp/suggestions/export",
            "/api/reports/current-stock",
            "/api/reports/inventory-value",
            "/api/reports/quality-status",
            "/api/reports/mrp-shortage",
            "/api/reports/low-stock-alert",
            "/api/reports/stock-by-zone",
            "/api/reports/bin-stock-summary",
            "/api/reports/batch-stock-summary",
            "/api/reports/data-consistency",
            "/api/reports/refresh",
            "/api/reports/current-stock/export",
        ] {
            assert!(
                MRP_REPORTING_OPENAPI_JSON.contains(path),
                "missing Phase 9 OpenAPI path: {path}"
            );
        }

        for tag in [
            "MRP",
            "MRP Suggestions",
            "Reports",
            "Report Export",
            "Materialized Views",
            "Data Consistency",
        ] {
            assert!(
                MRP_REPORTING_OPENAPI_JSON.contains(tag),
                "missing Phase 9 OpenAPI tag: {tag}"
            );
        }
    }

    #[tokio::test]
    async fn build_router_does_not_register_duplicate_routes() {
        let _router = build_router(test_state());
    }

    #[tokio::test]
    async fn audit_logs_route_is_registered_behind_auth() {
        let router = build_router(test_state());
        let status = send(
            &router,
            axum::http::Method::GET,
            "/api/system/audit-logs?page=1&page_size=1",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_current_grant_routes_are_registered_behind_auth() {
        let router = build_router(test_state());

        for path in ["/api/auth/roles", "/api/auth/permissions"] {
            let no_token = send(&router, axum::http::Method::GET, path, None).await;
            assert_eq!(no_token, StatusCode::UNAUTHORIZED, "path: {path}");
        }
    }

    #[tokio::test]
    async fn system_user_and_role_routes_are_registered_behind_auth() {
        let router = build_router(test_state());

        for path in [
            "/api/system/users?page=1&page_size=1",
            "/api/system/roles?page=1&page_size=1",
        ] {
            let no_token = send(&router, axum::http::Method::GET, path, None).await;
            assert_eq!(no_token, StatusCode::UNAUTHORIZED, "path: {path}");

            let read_token = issue_token(vec!["WMS_USER"], vec!["master-data:read"]);
            let not_admin = send(&router, axum::http::Method::GET, path, Some(&read_token)).await;
            assert_eq!(not_admin, StatusCode::FORBIDDEN, "path: {path}");
        }
    }

    #[tokio::test]
    async fn permission_guard_enforces_no_token_read_write_and_admin_access() {
        let router = test_router(test_state());

        let no_token = send(&router, axum::http::Method::GET, "/read", None).await;
        assert_eq!(no_token, StatusCode::UNAUTHORIZED);

        let read_token = issue_token(vec![], vec!["master-data:read"]);
        let read_ok = send(&router, axum::http::Method::GET, "/read", Some(&read_token)).await;
        assert_eq!(read_ok, StatusCode::OK);

        let read_only_write = send(
            &router,
            axum::http::Method::POST,
            "/write",
            Some(&read_token),
        )
        .await;
        assert_eq!(read_only_write, StatusCode::FORBIDDEN);

        let admin_token = issue_token(vec!["ADMIN"], vec![]);
        let admin_write = send(
            &router,
            axum::http::Method::POST,
            "/write",
            Some(&admin_token),
        )
        .await;
        assert_eq!(admin_write, StatusCode::OK);
    }

    #[tokio::test]
    async fn permission_revocation_requires_a_new_short_lived_access_token() {
        let router = test_router(test_state());

        let token_issued_before_revoke = issue_token(vec![], vec!["master-data:read"]);
        let still_accepted = send(
            &router,
            axum::http::Method::GET,
            "/read",
            Some(&token_issued_before_revoke),
        )
        .await;
        assert_eq!(still_accepted, StatusCode::OK);

        let token_issued_after_revoke = issue_token(vec![], vec![]);
        let revoked_permission_rejected = send(
            &router,
            axum::http::Method::GET,
            "/read",
            Some(&token_issued_after_revoke),
        )
        .await;
        assert_eq!(revoked_permission_rejected, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn inventory_read_permission_does_not_grant_history_cost_report_or_write_access() {
        let router = inventory_permission_router(test_state());
        let read_token = issue_token(vec![], vec!["inventory:read"]);

        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/current",
                Some(&read_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/transactions",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/batches",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/batches/BATCH-001/history",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/materials/RM001/map-history",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/stock-by-zone",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/post",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn batch_read_permission_does_not_grant_batch_history_access() {
        let router = inventory_permission_router(test_state());
        let batch_read_token = issue_token(vec![], vec!["batch:read"]);

        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/batches",
                Some(&batch_read_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/batches/BATCH-001/history",
                Some(&batch_read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn inventory_transfer_permission_does_not_grant_post_access() {
        let router = inventory_permission_router(test_state());
        let transfer_token = issue_token(vec![], vec!["inventory:transfer"]);

        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/transfer",
                Some(&transfer_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/post",
                Some(&transfer_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn phase9_permissions_are_route_specific() {
        let router = phase9_permission_router(test_state());

        let mrp_run_token = issue_token(vec![], vec!["mrp:run"]);
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/mrp/run",
                Some(&mrp_run_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/mrp/runs",
                Some(&mrp_run_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );

        let report_read_token = issue_token(vec![], vec!["report:read"]);
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/reports/current-stock",
                Some(&report_read_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/reports/refresh",
                Some(&report_read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );

        let report_export_token = issue_token(vec![], vec!["report:export"]);
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/reports/current-stock/export",
                Some(&report_export_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/reports/current-stock",
                Some(&report_export_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );

        let planner_token = issue_token(
            vec![],
            vec!["mrp:read", "mrp:suggestion-read", "mrp:suggestion-confirm"],
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/mrp/suggestions",
                Some(&planner_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/mrp/suggestions/1/confirm",
                Some(&planner_token)
            )
            .await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn inventory_count_permissions_are_independent() {
        let router = inventory_permission_router(test_state());
        let read_token = issue_token(vec![], vec!["inventory-count:read"]);
        let write_token = issue_token(vec![], vec!["inventory-count:write"]);
        let submit_token = issue_token(vec![], vec!["inventory-count:submit"]);
        let approve_token = issue_token(vec![], vec!["inventory-count:approve"]);
        let post_token = issue_token(vec![], vec!["inventory-count:post"]);
        let close_token = issue_token(vec![], vec!["inventory-count:close"]);

        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/counts",
                Some(&read_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::GET,
                "/counts/CNT-1/differences",
                Some(&read_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts",
                Some(&read_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts/CNT-1/generate-lines",
                Some(&write_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::PATCH,
                "/counts/CNT-1/lines",
                Some(&write_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts/CNT-1/submit",
                Some(&write_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts/CNT-1/submit",
                Some(&submit_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts/CNT-1/approve",
                Some(&approve_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts/CNT-1/post",
                Some(&post_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/counts/CNT-1/close",
                Some(&close_token)
            )
            .await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn order_receipt_and_shipment_permissions_are_separate_from_order_write() {
        let router = order_permission_router(test_state());
        let purchaser_token = issue_token(vec![], vec!["purchase:write"]);
        let warehouse_token = issue_token(vec![], vec!["purchase:receipt", "sales:shipment"]);
        let sales_token = issue_token(vec![], vec!["sales:write"]);

        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/purchase-orders",
                Some(&purchaser_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::PATCH,
                "/purchase-orders/PO-1",
                Some(&purchaser_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::PATCH,
                "/purchase-orders/PO-1",
                Some(&warehouse_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/purchase-orders/PO-1/receipt",
                Some(&purchaser_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/purchase-orders/PO-1/receipt",
                Some(&warehouse_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/sales-orders",
                Some(&sales_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::PATCH,
                "/sales-orders/SO-1",
                Some(&sales_token)
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::PATCH,
                "/sales-orders/SO-1",
                Some(&warehouse_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/sales-orders/SO-1/shipment",
                Some(&sales_token)
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &router,
                axum::http::Method::POST,
                "/sales-orders/SO-1/shipment",
                Some(&warehouse_token)
            )
            .await,
            StatusCode::OK
        );
    }
}
