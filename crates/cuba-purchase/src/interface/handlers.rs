use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use cuba_shared::{ApiResponse, AppResult, AppState, CurrentUser, write_audit_event};

use crate::{
    application::{
        PurchaseOrderClosed, PurchaseOrderCreated, PurchaseOrderDetail, PurchaseOrderService,
        PurchaseOrderSummary, PurchaseOrderUpdated, PurchaseReceiptPosted,
    },
    infrastructure::PostgresPurchaseOrderRepository,
    interface::dto::{
        CreatePurchaseOrderRequest, PostPurchaseReceiptRequest, UpdatePurchaseOrderRequest,
    },
};

fn service(state: &AppState) -> PurchaseOrderService {
    let repo = std::sync::Arc::new(PostgresPurchaseOrderRepository::new(state.db_pool.clone()));
    PurchaseOrderService::new(repo)
}

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<CreatePurchaseOrderRequest>,
) -> AppResult<Json<ApiResponse<PurchaseOrderCreated>>> {
    let command = crate::application::CreatePurchaseOrderCommand {
        supplier_id: request.supplier_id,
        expected_date: request.expected_date,
        remark: request.remark,
        lines: request
            .lines
            .into_iter()
            .map(|line| crate::application::CreatePurchaseOrderLineCommand {
                line_no: line.line_no,
                material_id: line.material_id,
                ordered_qty: line.ordered_qty,
                unit_price: line.unit_price,
                expected_bin: line.expected_bin,
            })
            .collect(),
    };

    let result = service(&state).create_order(command, user.username).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Query(query): Query<crate::application::PurchaseOrderQuery>,
) -> AppResult<Json<ApiResponse<Vec<PurchaseOrderSummary>>>> {
    let result = service(&state).list_orders(query).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_purchase_order(
    State(state): State<AppState>,
    Path(po_id): Path<String>,
) -> AppResult<Json<ApiResponse<PurchaseOrderDetail>>> {
    let result = service(&state).get_order(po_id).await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn update_purchase_order(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(po_id): Path<String>,
    Json(request): Json<UpdatePurchaseOrderRequest>,
) -> AppResult<Json<ApiResponse<PurchaseOrderUpdated>>> {
    let command = crate::application::UpdatePurchaseOrderCommand {
        po_id,
        supplier_id: request.supplier_id,
        expected_date: request.expected_date,
        remark: request.remark,
        lines: request.lines.map(|lines| {
            lines
                .into_iter()
                .map(|line| crate::application::CreatePurchaseOrderLineCommand {
                    line_no: line.line_no,
                    material_id: line.material_id,
                    ordered_qty: line.ordered_qty,
                    unit_price: line.unit_price,
                    expected_bin: line.expected_bin,
                })
                .collect()
        }),
    };

    let result = service(&state).update_order(command, user.username).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn post_purchase_receipt(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(po_id): Path<String>,
    Json(request): Json<PostPurchaseReceiptRequest>,
) -> AppResult<Json<ApiResponse<PurchaseReceiptPosted>>> {
    let record_id = po_id.clone();
    let command = crate::application::PostPurchaseReceiptCommand {
        po_id,
        posting_date: request.posting_date,
        remark: request.remark,
        lines: request
            .lines
            .into_iter()
            .map(|line| crate::application::PostPurchaseReceiptLineCommand {
                line_no: line.line_no,
                receipt_qty: line.receipt_qty,
                batch_number: line.batch_number,
                to_bin: line.to_bin,
            })
            .collect(),
    };

    let result = service(&state)
        .post_receipt(command, user.username.clone())
        .await?;
    write_audit_event(
        &state.db_pool,
        Some(user.user_id),
        "PURCHASE_RECEIPT_POST",
        Some("wms.wms_purchase_orders_h"),
        Some(&record_id),
        serde_json::to_value(&result).ok(),
    )
    .await;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn close_purchase_order(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(po_id): Path<String>,
) -> AppResult<Json<ApiResponse<PurchaseOrderClosed>>> {
    let result = service(&state).close_order(po_id, user.username).await?;

    Ok(Json(ApiResponse::ok(result)))
}
