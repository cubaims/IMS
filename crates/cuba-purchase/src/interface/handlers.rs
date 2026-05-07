use axum::{
    extract::{Path, Query, State},
    Json,
};
use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::PurchaseOrderService,
    infrastructure::PostgresPurchaseOrderRepository,
    interface::dto::{CreatePurchaseOrderRequest, PostPurchaseReceiptRequest},
};

fn service(state: &AppState) -> PurchaseOrderService {
    let repo = std::sync::Arc::new(PostgresPurchaseOrderRepository::new(state.db_pool.clone()));
    PurchaseOrderService::new(repo)
}

fn operator_from_headers(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-user-name")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("API")
        .to_string()
}

pub async fn create_purchase_order(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreatePurchaseOrderRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = crate::application::CreatePurchaseOrderCommand {
        supplier_id: request.supplier_id,
        expected_date: request.expected_date,
        remark: request.remark,
        lines: request.lines.into_iter().map(|line| crate::application::CreatePurchaseOrderLineCommand {
            line_no: line.line_no,
            material_id: line.material_id,
            ordered_qty: line.ordered_qty,
            unit_price: line.unit_price,
            expected_bin: line.expected_bin,
        }).collect(),
    };

    let result = service(&state)
        .create_order(command, operator_from_headers(&headers))
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Query(query): Query<crate::application::PurchaseOrderQuery>,
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
    headers: axum::http::HeaderMap,
    Path(po_id): Path<String>,
    Json(request): Json<PostPurchaseReceiptRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let command = crate::application::PostPurchaseReceiptCommand {
        po_id,
        posting_date: request.posting_date,
        remark: request.remark,
        lines: request.lines.into_iter().map(|line| crate::application::PostPurchaseReceiptLineCommand {
            line_no: line.line_no,
            receipt_qty: line.receipt_qty,
            batch_number: line.batch_number,
            to_bin: line.to_bin,
        }).collect(),
    };

    let result = service(&state)
        .post_receipt(command, operator_from_headers(&headers))
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn close_purchase_order(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(po_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let result = service(&state)
        .close_order(po_id, operator_from_headers(&headers))
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}