use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use cuba_shared::{ApiResponse, AppResult, AppState, CurrentUser, write_audit_event};

use crate::{
    application::{
        CreateSalesOrderCommand, CreateSalesOrderLineCommand, PostSalesShipmentCommand,
        PostSalesShipmentLineCommand, PreviewSalesFefoPickCommand, PreviewSalesFefoPickLineCommand,
        SalesOrderQuery, SalesOrderService, UpdateSalesOrderCommand,
    },
    infrastructure::PostgresSalesOrderRepository,
    interface::dto::{
        CreateSalesOrderRequest, PostSalesShipmentRequest, PreviewSalesFefoPickRequest,
        UpdateSalesOrderRequest,
    },
};

fn service(state: &AppState) -> SalesOrderService {
    let repository = PostgresSalesOrderRepository::new(state.db_pool.clone());
    SalesOrderService::new(Arc::new(repository))
}

pub async fn create_sales_order(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
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

    let result = service(&state).create_order(command, user.username).await?;

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

pub async fn update_sales_order(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(so_id): Path<String>,
    Json(request): Json<UpdateSalesOrderRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = UpdateSalesOrderCommand {
        so_id,
        customer_id: request.customer_id,
        required_date: request.required_date,
        remark: request.remark,
        lines: request.lines.map(|lines| {
            lines
                .into_iter()
                .map(|line| CreateSalesOrderLineCommand {
                    line_no: line.line_no,
                    material_id: line.material_id,
                    ordered_qty: line.ordered_qty,
                    unit_price: line.unit_price,
                    from_bin: line.from_bin,
                })
                .collect()
        }),
    };

    let result = service(&state).update_order(command, user.username).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn post_sales_shipment(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(so_id): Path<String>,
    Json(request): Json<PostSalesShipmentRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let record_id = so_id.clone();
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
        .post_shipment(command, user.username.clone())
        .await?;
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "SALES_SHIPMENT_POST",
        Some("wms.wms_sales_orders_h"),
        Some(&record_id),
        Some(result.clone()),
    )
    .await;

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
    Extension(user): Extension<CurrentUser>,
    Path(so_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state).close_order(so_id, user.username).await?;

    Ok(Json(ApiResponse::ok(result)))
}
