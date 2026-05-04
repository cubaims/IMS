use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{
        CreateSalesOrderCommand, CreateSalesOrderLineCommand, PostSalesShipmentCommand,
        PostSalesShipmentLineCommand, PreviewSalesFefoPickCommand, PreviewSalesFefoPickLineCommand,
        SalesOrderQuery, SalesOrderService,
    },
    infrastructure::PostgresSalesOrderRepository,
    interface::dto::{
        CreateSalesOrderRequest, PostSalesShipmentRequest, PreviewSalesFefoPickRequest,
    },
};

fn service(state: &AppState) -> SalesOrderService {
    let repository = PostgresSalesOrderRepository::new(state.db_pool.clone());
    SalesOrderService::new(Arc::new(repository))
}

fn current_operator() -> String {
    // Phase 5 先用占位值。
    // Phase 2 权限中间件完成后，从 CurrentUser 中读取 username。
    "api".to_string()
}

pub async fn create_sales_order(
    State(state): State<AppState>,
    Json(request): Json<CreateSalesOrderRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = CreateSalesOrderCommand {
        customer_id: request.customer_id,
        required_date: request.required_date,
        remark: request.remark,
        lines: request
            .lines
            .into_iter()
            .map(|line| CreateSalesOrderLineCommand {
                line_no: line.line_no,
                material_id: line.material_id,
                ordered_qty: line.ordered_qty,
                unit_price: line.unit_price,
                from_bin: line.from_bin,
            })
            .collect(),
    };

    let result = service(&state)
        .create_order(command, current_operator())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_sales_orders(
    State(state): State<AppState>,
    Query(query): Query<SalesOrderQuery>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state).list_orders(query).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_sales_order(
    State(state): State<AppState>,
    Path(so_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state).get_order(so_id).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn post_sales_shipment(
    State(state): State<AppState>,
    Path(so_id): Path<String>,
    Json(request): Json<PostSalesShipmentRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = PostSalesShipmentCommand {
        so_id,
        posting_date: request.posting_date,
        pick_strategy: request.pick_strategy,
        remark: request.remark,
        lines: request
            .lines
            .into_iter()
            .map(|line| PostSalesShipmentLineCommand {
                line_no: line.line_no,
                shipment_qty: line.shipment_qty,
                batch_number: line.batch_number,
                from_bin: line.from_bin,
            })
            .collect(),
    };

    let result = service(&state)
        .post_shipment(command, current_operator())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn preview_sales_fefo_pick(
    State(state): State<AppState>,
    Path(so_id): Path<String>,
    Json(request): Json<PreviewSalesFefoPickRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = PreviewSalesFefoPickCommand {
        so_id,
        lines: request
            .lines
            .into_iter()
            .map(|line| PreviewSalesFefoPickLineCommand {
                line_no: line.line_no,
                shipment_qty: line.shipment_qty,
            })
            .collect(),
    };

    let result = service(&state).preview_fefo_pick(command).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn close_sales_order(
    State(state): State<AppState>,
    Path(so_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state)
        .close_order(so_id, current_operator())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}
