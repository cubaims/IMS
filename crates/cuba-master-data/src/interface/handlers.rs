use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;

use cuba_shared::{ApiResponse, AppResult, AppState};

use crate::{
    application::MasterDataService,
    infrastructure::PostgresMasterDataRepository,
    interface::dto::{
        BomExplosionPreviewQuery, CreateBomComponentCommand, CreateBomHeaderCommand,
        CreateCustomerCommand, CreateDefectCodeCommand, CreateInspectionCharCommand,
        CreateMaterialCommand, CreateMaterialSupplierCommand, CreateProductVariantCommand,
        CreateStorageBinCommand, CreateSupplierCommand, CreateWorkCenterCommand, MasterDataQuery,
        UpdateBomComponentCommand, UpdateBomHeaderCommand, UpdateCustomerCommand,
        UpdateDefectCodeCommand, UpdateInspectionCharCommand, UpdateMaterialCommand,
        UpdateMaterialSupplierCommand, UpdateProductVariantCommand, UpdateStorageBinCommand,
        UpdateSupplierCommand, UpdateWorkCenterCommand,
    },
};

fn service(state: &AppState) -> MasterDataService {
    let repo = Arc::new(PostgresMasterDataRepository::new(state.db_pool.clone()));

    MasterDataService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo,
    )
}

pub async fn list_materials(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_materials(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_material(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_material(&material_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_material(
    State(state): State<AppState>,
    Json(command): Json<CreateMaterialCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_material(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_material(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
    Json(command): Json<UpdateMaterialCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_material(&material_id, command)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_material(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_material(&material_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_material(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_material(&material_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_bins(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_bins(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bin(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_bin(&bin_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_bin(
    State(state): State<AppState>,
    Json(command): Json<CreateStorageBinCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_bin(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bin(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
    Json(command): Json<UpdateStorageBinCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).update_bin(&bin_code, command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_bin(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_bin(&bin_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_bin(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_bin(&bin_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bin_capacity_utilization(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .get_bin_capacity_utilization(&bin_code)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_suppliers(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_suppliers(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_supplier(&supplier_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_supplier(
    State(state): State<AppState>,
    Json(command): Json<CreateSupplierCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_supplier(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<String>,
    Json(command): Json<UpdateSupplierCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_supplier(&supplier_id, command)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_supplier(&supplier_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_supplier(&supplier_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_customers(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_customer(
    State(state): State<AppState>,
    Path(customer_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_customer(&customer_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_customer(
    State(state): State<AppState>,
    Json(command): Json<CreateCustomerCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_customer(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_customer(
    State(state): State<AppState>,
    Path(customer_id): Path<String>,
    Json(command): Json<UpdateCustomerCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_customer(&customer_id, command)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_customer(
    State(state): State<AppState>,
    Path(customer_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_customer(&customer_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_customer(
    State(state): State<AppState>,
    Path(customer_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_customer(&customer_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_material_suppliers(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .list_material_suppliers(&material_id)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_material_supplier(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
    Json(mut command): Json<CreateMaterialSupplierCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    command.material_id = material_id;

    let data = service(&state).create_material_supplier(command).await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_material_supplier(
    State(state): State<AppState>,
    Path((material_id, supplier_id)): Path<(String, String)>,
    Json(command): Json<UpdateMaterialSupplierCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_material_supplier(&material_id, &supplier_id, command)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn set_primary_supplier(
    State(state): State<AppState>,
    Path((material_id, supplier_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .set_primary_supplier(&material_id, &supplier_id)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn remove_material_supplier(
    State(state): State<AppState>,
    Path((material_id, supplier_id)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .remove_material_supplier(&material_id, &supplier_id)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_variants(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_variants(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_variant(
    State(state): State<AppState>,
    Path(variant_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_variant(&variant_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_variant(
    State(state): State<AppState>,
    Json(command): Json<CreateProductVariantCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_variant(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_variant(
    State(state): State<AppState>,
    Path(variant_code): Path<String>,
    Json(command): Json<UpdateProductVariantCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_variant(&variant_code, command)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_variant(
    State(state): State<AppState>,
    Path(variant_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_variant(&variant_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_variant(
    State(state): State<AppState>,
    Path(variant_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_variant(&variant_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_boms(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_boms(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_bom(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_bom(
    State(state): State<AppState>,
    Json(command): Json<CreateBomHeaderCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_bom(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
    Json(command): Json<UpdateBomHeaderCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).update_bom(&bom_id, command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_bom(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_bom(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_bom_components(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_components(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn add_bom_component(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
    Json(mut command): Json<CreateBomComponentCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    command.bom_id = bom_id;

    let data = service(&state).add_component(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bom_component(
    State(state): State<AppState>,
    Path(component_id): Path<i64>,
    Json(command): Json<UpdateBomComponentCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_component(component_id, command)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn remove_bom_component(
    State(state): State<AppState>,
    Path(component_id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).remove_component(component_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bom_tree(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_bom_tree(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn validate_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).validate_bom(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn preview_bom_explosion(
    State(state): State<AppState>,
    Query(query): Query<BomExplosionPreviewQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .preview_bom_explosion(&query.material_id, query.quantity, query.variant_code)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_work_centers(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_work_centers(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_work_center(
    State(state): State<AppState>,
    Path(work_center_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_work_center(&work_center_id).await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_work_center(
    State(state): State<AppState>,
    Json(command): Json<CreateWorkCenterCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_work_center(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_work_center(
    State(state): State<AppState>,
    Path(work_center_id): Path<String>,
    Json(command): Json<UpdateWorkCenterCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_work_center(&work_center_id, command)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_work_center(
    State(state): State<AppState>,
    Path(work_center_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .activate_work_center(&work_center_id)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_work_center(
    State(state): State<AppState>,
    Path(work_center_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .deactivate_work_center(&work_center_id)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_inspection_chars(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_inspection_chars(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_inspection_char(
    State(state): State<AppState>,
    Path(char_id): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_inspection_char(&char_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_inspection_char(
    State(state): State<AppState>,
    Json(command): Json<CreateInspectionCharCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_inspection_char(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_inspection_char(
    State(state): State<AppState>,
    Path(char_id): Path<String>,
    Json(command): Json<UpdateInspectionCharCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_inspection_char(&char_id, command)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_defect_codes(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).list_defect_codes(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_defect_code(
    State(state): State<AppState>,
    Path(defect_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).get_defect_code(&defect_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_defect_code(
    State(state): State<AppState>,
    Json(command): Json<CreateDefectCodeCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).create_defect_code(command).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_defect_code(
    State(state): State<AppState>,
    Path(defect_code): Path<String>,
    Json(command): Json<UpdateDefectCodeCommand>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state)
        .update_defect_code(&defect_code, command)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_defect_code(
    State(state): State<AppState>,
    Path(defect_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).activate_defect_code(&defect_code).await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_defect_code(
    State(state): State<AppState>,
    Path(defect_code): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = service(&state).deactivate_defect_code(&defect_code).await?;

    Ok(Json(ApiResponse::ok(data)))
}
