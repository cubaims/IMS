use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{
        CreatePurchaseOrderCommand, CreatePurchaseOrderLineCommand, PostPurchaseReceiptCommand,
        PostPurchaseReceiptLineCommand, PurchaseOrderQuery, PurchaseOrderService,
    },
    infrastructure::PostgresPurchaseOrderRepository,
    interface::dto::{CreatePurchaseOrderRequest, PostPurchaseReceiptRequest},
};

fn service(state: &AppState) -> PurchaseOrderService {
    let repository = PostgresPurchaseOrderRepository::new(state.db_pool.clone());
    PurchaseOrderService::new(Arc::new(repository))
}

fn current_operator() -> String {
    // Phase 5 先用占位值。
    // Phase 2 权限中间件完成后，从 CurrentUser 中读取 username。
    "api".to_string()
}

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Json(request): Json<CreatePurchaseOrderRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = CreatePurchaseOrderCommand {
        supplier_id: request.supplier_id,
        expected_date: request.expected_date,
        remark: request.remark,
        lines: request
            .lines
            .into_iter()
            .map(|line| CreatePurchaseOrderLineCommand {
                line_no: line.line_no,
                material_id: line.material_id,
                ordered_qty: line.ordered_qty,
                unit_price: line.unit_price,
                expected_bin: line.expected_bin,
            })
            .collect(),
    };

    let result = service(&state)
        .create_order(command, current_operator())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Query(query): Query<PurchaseOrderQuery>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state).list_orders(query).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_purchase_order(
    State(state): State<AppState>,
    Path(po_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state).get_order(po_id).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn post_purchase_receipt(
    State(state): State<AppState>,
    Path(po_id): Path<String>,
    Json(request): Json<PostPurchaseReceiptRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = PostPurchaseReceiptCommand {
        po_id,
        posting_date: request.posting_date,
        remark: request.remark,
        lines: request
            .lines
            .into_iter()
            .map(|line| PostPurchaseReceiptLineCommand {
                line_no: line.line_no,
                receipt_qty: line.receipt_qty,
                batch_number: line.batch_number,
                to_bin: line.to_bin,
            })
            .collect(),
    };

    let result = service(&state)
        .post_receipt(command, current_operator())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn close_purchase_order(
    State(state): State<AppState>,
    Path(po_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state)
        .close_order(po_id, current_operator())
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}
