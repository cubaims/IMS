use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::{
        BomExplosionCommand, CompleteProductionOrderCommand, CreateProductionOrderCommand,
        ProductionOrderQuery, ProductionVarianceQuery, ReleaseProductionOrderCommand,
    },
    domain::ProductionOrderStatus,
    infrastructure::PostgresProductionRepository,
};

use super::dto::{
    BomExplosionPreviewRequest, CancelProductionOrderRequest, CloseProductionOrderRequest,
    CompleteProductionOrderRequest, CreateProductionOrderRequest, CreatedProductionOrderResponse,
    ProductionActionResponse, ProductionOrderListQuery, ProductionVarianceListQuery,
    ReleaseProductionOrderRequest,
};

fn production_service(state: &AppState) -> crate::application::ProductionService {
    let repo = Arc::new(PostgresProductionRepository::new(state.db_pool.clone()));

    crate::application::ProductionService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    )
}

fn status_text(status: ProductionOrderStatus) -> String {
    status.as_db_text().to_string()
}

pub async fn preview_bom_explosion(
    State(state): State<AppState>,
    Json(req): Json<BomExplosionPreviewRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::BomExplosionResult>>> {
    let service = production_service(&state);

    let result = service
        .explode_bom(BomExplosionCommand {
            variant_code: req.variant_code,
            finished_material_id: req.finished_material_id,
            quantity: req.quantity,
            merge_components: req.merge_components,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn create_production_order(
    State(state): State<AppState>,
    Json(req): Json<CreateProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<CreatedProductionOrderResponse>>> {
    let service = production_service(&state);

    let order_id = service
        .create_order(CreateProductionOrderCommand {
            variant_code: req.variant_code,
            finished_material_id: req.finished_material_id,
            bom_id: req.bom_id,
            planned_qty: req.planned_qty,
            work_center_id: req.work_center_id,
            planned_start_date: req.planned_start_date,
            planned_end_date: req.planned_end_date,
            remark: req.remark,
            created_by: Some("API".to_string()),
        })
        .await?;

    Ok(Json(ApiResponse::ok(CreatedProductionOrderResponse {
        order_id: order_id.0,
    })))
}

pub async fn list_production_orders(
    State(state): State<AppState>,
    Query(query): Query<ProductionOrderListQuery>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::ProductionOrder>>>> {
    let service = production_service(&state);

    let result = service
        .list_orders(ProductionOrderQuery {
            order_id: query.order_id,
            variant_code: query.variant_code,
            finished_material_id: query.finished_material_id,
            status: query.status,
            page: query.page,
            page_size: query.page_size,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::ProductionOrder>>> {
    let service = production_service(&state);
    let result = service.get_order(&order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_production_order_components(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::ProductionOrderLine>>>> {
    let service = production_service(&state);
    let result = service.list_order_lines(&order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn release_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(req): Json<ReleaseProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<ProductionActionResponse>>> {
    let service = production_service(&state);

    let result = service
        .release_order(ReleaseProductionOrderCommand {
            order_id,
            remark: req.remark,
            operator: Some("API".to_string()),
        })
        .await?;

    Ok(Json(ApiResponse::ok(ProductionActionResponse {
        order_id: result.order_id.0,
        status: status_text(result.status),
    })))
}

pub async fn cancel_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(_req): Json<CancelProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<ProductionActionResponse>>> {
    let service = production_service(&state);

    let result = service
        .cancel_order(&order_id, Some("API".to_string()))
        .await?;

    Ok(Json(ApiResponse::ok(ProductionActionResponse {
        order_id: result.order_id.0,
        status: status_text(result.status),
    })))
}

pub async fn close_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(_req): Json<CloseProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<ProductionActionResponse>>> {
    let service = production_service(&state);

    let result = service
        .close_order(&order_id, Some("API".to_string()))
        .await?;

    Ok(Json(ApiResponse::ok(ProductionActionResponse {
        order_id: result.order_id.0,
        status: status_text(result.status),
    })))
}

pub async fn complete_production_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(req): Json<CompleteProductionOrderRequest>,
) -> AppResult<Json<ApiResponse<crate::domain::ProductionCompleteResult>>> {
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
            operator: Some("API".to_string()),
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_production_genealogy(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BatchGenealogy>>>> {
    let service = production_service(&state);
    let result = service.get_genealogy(&order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_finished_batch_components(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BatchGenealogy>>>> {
    let repo = PostgresProductionRepository::new(state.db_pool.clone());

    let result = crate::application::BatchGenealogyRepository::find_components_by_finished_batch(
        &repo,
        &batch_number,
    )
    .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_component_batch_where_used(
    State(state): State<AppState>,
    Path(batch_number): Path<String>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::BatchGenealogy>>>> {
    let repo = PostgresProductionRepository::new(state.db_pool.clone());

    let result = crate::application::BatchGenealogyRepository::find_where_used_by_component_batch(
        &repo,
        &batch_number,
    )
    .await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn get_production_variance(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> AppResult<Json<ApiResponse<crate::domain::ProductionVariance>>> {
    let service = production_service(&state);
    let result = service.get_variance(&order_id).await?;

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn list_production_variances(
    State(state): State<AppState>,
    Query(query): Query<ProductionVarianceListQuery>,
) -> AppResult<Json<ApiResponse<Vec<crate::domain::ProductionVariance>>>> {
    let service = production_service(&state);

    let result = service
        .list_variances(ProductionVarianceQuery {
            order_id: query.order_id,
            variant_code: query.variant_code,
            only_over_budget: query.only_over_budget,
            page: query.page,
            page_size: query.page_size,
        })
        .await?;

    Ok(Json(ApiResponse::ok(result)))
}
