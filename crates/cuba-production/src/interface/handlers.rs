use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};

use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{
        CompleteProductionOrderCommand, CreateProductionOrderCommand,
        ListProductionOrdersQuery, ListProductionVariancesQuery,
        PreviewBomExplosionCommand, ProductionService,
        ReleaseProductionOrderCommand,
    },
    infrastructure::PostgresProductionRepository,
    interface::dto::{
        BomExplosionPreviewRequest, CancelProductionOrderRequest,
        CloseProductionOrderRequest, CompleteProductionOrderRequest,
        CreateProductionOrderRequest, ListProductionOrdersRequest,
        ListProductionVariancesRequest, ReleaseProductionOrderRequest,
    },
};

fn production_service(state: &AppState) -> ProductionService {
    let repo = Arc::new(PostgresProductionRepository::new(state.db_pool.clone()));

    ProductionService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    )
}

pub async fn preview_bom_explosion(
    State(state): State<AppState>,
    Json(req): Json<BomExplosionPreviewRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service
        .preview_bom_explosion(PreviewBomExplosionCommand {
            variant_code: req.variant_code,
            finished_material_id: req.finished_material_id,
            quantity: req.quantity,
            merge_components: req.merge_components,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_production_orders(
    State(state): State<AppState>,
    Query(req): Query<ListProductionOrdersRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service
        .list_orders(ListProductionOrdersQuery {
            status: req.status,
            variant_code: req.variant_code,
            finished_material_id: req.finished_material_id,
            work_center_id: req.work_center_id,
            date_from: req.date_from,
            date_to: req.date_to,
            page: req.page,
            page_size: req.page_size,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn create_production_order(
    State(state): State<AppState>,
    Json(req): Json<CreateProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<crate::application::CreateProductionOrderResult>>> {
    let service = production_service(&state);

    let result = service
        .create_order(CreateProductionOrderCommand {
            variant_code: req.variant_code,
            finished_material_id: req.finished_material_id,
            bom_id: req.bom_id,
            planned_qty: req.planned_qty,
            work_center_id: req.work_center_id,
            planned_start_date: req.planned_start_date,
            planned_end_date: req.planned_end_date,
            remark: req.remark,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service.get_order(order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn release_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(req): Json<ReleaseProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<crate::application::ReleaseProductionOrderResult>>> {
    let service = production_service(&state);

    let result = service
        .release_order(ReleaseProductionOrderCommand {
            order_id,
            remark: req.remark,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn cancel_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(req): Json<CancelProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service.cancel_order(order_id, req.remark).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn close_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(req): Json<CloseProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service.close_order(order_id, req.remark).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn complete_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(req): Json<CompleteProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<crate::application::ProductionCompleteAppResult>>> {
    let service = production_service(&state);

    let result = service
        .complete_order(CompleteProductionOrderCommand {
            order_id,
            completed_qty: req.completed_qty,
            finished_batch_number: req.finished_batch_number,
            finished_to_bin: req.finished_to_bin,
            posting_date: req.posting_date,
            pick_strategy: req.pick_strategy,
            remark: req.remark,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_order_components(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service.get_order_components(order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_order_genealogy(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service.get_order_genealogy(order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_components_by_finished_batch(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service
        .get_components_by_finished_batch(batch_number)
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_where_used_by_component_batch(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service
        .get_where_used_by_component_batch(batch_number)
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_order_variance(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service.get_order_variance(order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_production_variances(
    State(state): State<AppState>,
    Query(req): Query<ListProductionVariancesRequest>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let service = production_service(&state);

    let result = service
        .list_variances(ListProductionVariancesQuery {
            order_id: req.order_id,
            variant_code: req.variant_code,
            date_from: req.date_from,
            date_to: req.date_to,
            only_over_budget: req.only_over_budget,
            page: req.page,
            page_size: req.page_size,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}