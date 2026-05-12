use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
};
use serde::Serialize;
use serde_json::Value;

use cuba_shared::{ApiResponse, AppResult, AppState, CurrentUser, write_audit_change};

use crate::{
    application::MasterDataService,
    infrastructure::PostgresMasterDataRepository,
    interface::dto::{
        BomExplosionPreviewQuery, BomExplosionPreviewRequest, CopyBomCommand,
        CreateBomComponentCommand, CreateBomHeaderCommand, CreateCustomerCommand,
        CreateDefectCodeCommand, CreateInspectionCharCommand, CreateMaterialCommand,
        CreateMaterialSupplierCommand, CreateProductVariantCommand, CreateStorageBinCommand,
        CreateSupplierCommand, CreateWorkCenterCommand, MasterDataQuery, UpdateBomComponentCommand,
        UpdateBomHeaderCommand, UpdateCustomerCommand, UpdateDefectCodeCommand,
        UpdateInspectionCharCommand, UpdateMaterialCommand, UpdateMaterialSupplierCommand,
        UpdateProductVariantCommand, UpdateStorageBinCommand, UpdateSupplierCommand,
        UpdateWorkCenterCommand,
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

async fn audit_master_data_change(
    state: &AppState,
    user: &CurrentUser,
    action: &str,
    table_name: &str,
    record_id: &str,
    data: impl Serialize,
) {
    let Ok(data) = serde_json::to_value(data) else {
        tracing::warn!(
            action,
            table_name,
            record_id,
            "failed to serialize master data audit payload"
        );
        return;
    };

    let (old_data, new_data): (Option<Value>, Option<Value>) = if action == "MASTER_DATA_DELETE" {
        (Some(data), None)
    } else {
        (None, Some(data))
    };

    write_audit_change(
        &state.db_pool,
        Some(user.user_id),
        action,
        table_name,
        record_id,
        old_data,
        new_data,
    )
    .await;
}

pub async fn list_materials(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_materials(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_material(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_material(&material_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_material(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateMaterialCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.material_id.clone();
    let data = service(&state).create_material(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_materials",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_material(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(material_id): Path<String>,
    Json(command): Json<UpdateMaterialCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_material(&material_id, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_materials",
        &material_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_material(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(material_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_material(&material_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_materials",
        &material_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_material(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(material_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_material(&material_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_materials",
        &material_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_bins(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_bins(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bin(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_bin(&bin_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_bin(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateStorageBinCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.bin_code.clone();
    let data = service(&state).create_bin(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_storage_bins",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bin(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bin_code): Path<String>,
    Json(command): Json<UpdateStorageBinCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).update_bin(&bin_code, command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_storage_bins",
        &bin_code,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_bin(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bin_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_bin(&bin_code).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_storage_bins",
        &bin_code,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_bin(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bin_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_bin(&bin_code).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_storage_bins",
        &bin_code,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bin_capacity_utilization(
    State(state): State<AppState>,
    Path(bin_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .get_bin_capacity_utilization(&bin_code)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_suppliers(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_suppliers(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_supplier(&supplier_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateSupplierCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.supplier_id.clone();
    let data = service(&state).create_supplier(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_suppliers",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(supplier_id): Path<String>,
    Json(command): Json<UpdateSupplierCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_supplier(&supplier_id, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_suppliers",
        &supplier_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(supplier_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_supplier(&supplier_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_suppliers",
        &supplier_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(supplier_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_supplier(&supplier_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_suppliers",
        &supplier_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_customers(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_customer(
    State(state): State<AppState>,
    Path(customer_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_customer(&customer_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_customer(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateCustomerCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.customer_id.clone();
    let data = service(&state).create_customer(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_customers",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_customer(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(customer_id): Path<String>,
    Json(command): Json<UpdateCustomerCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_customer(&customer_id, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_customers",
        &customer_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_customer(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(customer_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_customer(&customer_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_customers",
        &customer_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_customer(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(customer_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_customer(&customer_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_customers",
        &customer_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_material_suppliers(
    State(state): State<AppState>,
    Path(material_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .list_material_suppliers(&material_id)
        .await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_material_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(material_id): Path<String>,
    Json(mut command): Json<CreateMaterialSupplierCommand>,
) -> AppResult<impl IntoResponse> {
    command.material_id = material_id;
    let record_id = format!("{}:{}", command.material_id, command.supplier_id);

    let data = service(&state).create_material_supplier(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_material_suppliers",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_material_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((material_id, supplier_id)): Path<(String, String)>,
    Json(command): Json<UpdateMaterialSupplierCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = format!("{material_id}:{supplier_id}");
    let data = service(&state)
        .update_material_supplier(&material_id, &supplier_id, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_material_suppliers",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn set_primary_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((material_id, supplier_id)): Path<(String, String)>,
) -> AppResult<impl IntoResponse> {
    let record_id = format!("{material_id}:{supplier_id}");
    let data = service(&state)
        .set_primary_supplier(&material_id, &supplier_id)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_SET_PRIMARY",
        "mdm.mdm_material_suppliers",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn cancel_primary_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((material_id, supplier_id)): Path<(String, String)>,
) -> AppResult<impl IntoResponse> {
    let record_id = format!("{material_id}:{supplier_id}");
    let data = service(&state)
        .cancel_primary_supplier(&material_id, &supplier_id)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CANCEL_PRIMARY",
        "mdm.mdm_material_suppliers",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn remove_material_supplier(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((material_id, supplier_id)): Path<(String, String)>,
) -> AppResult<impl IntoResponse> {
    let record_id = format!("{material_id}:{supplier_id}");
    let data = service(&state)
        .remove_material_supplier(&material_id, &supplier_id)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DELETE",
        "mdm.mdm_material_suppliers",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_variants(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_variants(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_variant(
    State(state): State<AppState>,
    Path(variant_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_variant(&variant_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_variant(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateProductVariantCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.variant_code.clone();
    let data = service(&state).create_variant(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_product_variants",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_variant(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(variant_code): Path<String>,
    Json(command): Json<UpdateProductVariantCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_variant(&variant_code, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_product_variants",
        &variant_code,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_variant(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(variant_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_variant(&variant_code).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_product_variants",
        &variant_code,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_variant(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(variant_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_variant(&variant_code).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_product_variants",
        &variant_code,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_boms(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_boms(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_bom(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateBomHeaderCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.bom_id.clone();
    let data = service(&state).create_bom(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_bom_headers",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn copy_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(source_bom_id): Path<String>,
    Json(command): Json<CopyBomCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.target_bom_id.clone();
    let data = service(&state).copy_bom(&source_bom_id, command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_bom_headers",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bom_id): Path<String>,
    Json(command): Json<UpdateBomHeaderCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).update_bom(&bom_id, command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_bom_headers",
        &bom_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bom_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_bom(&bom_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_bom_headers",
        &bom_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bom_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_bom(&bom_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_bom_headers",
        &bom_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_bom_components(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_components(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn add_bom_component(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(bom_id): Path<String>,
    Json(mut command): Json<CreateBomComponentCommand>,
) -> AppResult<impl IntoResponse> {
    command.bom_id = bom_id;
    let record_id = command.bom_id.clone();

    let data = service(&state).add_component(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_bom_components",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bom_component(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(component_id): Path<i64>,
    Json(command): Json<UpdateBomComponentCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_component(component_id, command)
        .await?;
    let record_id = component_id.to_string();
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_bom_components",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_bom_component_for_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((bom_id, component_id)): Path<(String, i64)>,
    Json(command): Json<UpdateBomComponentCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_component_for_bom(&bom_id, component_id, command)
        .await?;
    let record_id = format!("{bom_id}:{component_id}");
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_bom_components",
        &record_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn remove_bom_component(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(component_id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).remove_component(component_id).await?;
    let record_id = component_id.to_string();
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DELETE",
        "mdm.mdm_bom_components",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn remove_bom_component_for_bom(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((bom_id, component_id)): Path<(String, i64)>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .remove_component_for_bom(&bom_id, component_id)
        .await?;
    let record_id = format!("{bom_id}:{component_id}");
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DELETE",
        "mdm.mdm_bom_components",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_bom_tree(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_bom_tree(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn validate_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).validate_bom(&bom_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn preview_bom_explosion(
    State(state): State<AppState>,
    Query(query): Query<BomExplosionPreviewQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .preview_bom_explosion(&query.material_id, query.quantity, query.variant_code)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn preview_bom_explosion_for_bom(
    State(state): State<AppState>,
    Path(bom_id): Path<String>,
    Json(request): Json<BomExplosionPreviewRequest>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .preview_bom_explosion_for_bom(&bom_id, request.quantity, request.variant_code)
        .await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_work_centers(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_work_centers(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_work_center(
    State(state): State<AppState>,
    Path(work_center_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_work_center(&work_center_id).await?;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_work_center(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateWorkCenterCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.work_center_id.clone();
    let data = service(&state).create_work_center(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_work_centers",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_work_center(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(work_center_id): Path<String>,
    Json(command): Json<UpdateWorkCenterCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_work_center(&work_center_id, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_work_centers",
        &work_center_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_work_center(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(work_center_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .activate_work_center(&work_center_id)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_work_centers",
        &work_center_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_work_center(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(work_center_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .deactivate_work_center(&work_center_id)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_work_centers",
        &work_center_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_inspection_chars(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_inspection_chars(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_inspection_char(
    State(state): State<AppState>,
    Path(char_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_inspection_char(&char_id).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_inspection_char(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateInspectionCharCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.char_id.clone();
    let data = service(&state).create_inspection_char(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_inspection_chars",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_inspection_char(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(char_id): Path<String>,
    Json(command): Json<UpdateInspectionCharCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_inspection_char(&char_id, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_inspection_chars",
        &char_id,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_inspection_char(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(char_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_inspection_char(&char_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_inspection_chars",
        &char_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_inspection_char(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(char_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_inspection_char(&char_id).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_inspection_chars",
        &char_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn list_defect_codes(
    State(state): State<AppState>,
    Query(query): Query<MasterDataQuery>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).list_defect_codes(query).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn get_defect_code(
    State(state): State<AppState>,
    Path(defect_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).get_defect_code(&defect_code).await?;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn create_defect_code(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(command): Json<CreateDefectCodeCommand>,
) -> AppResult<impl IntoResponse> {
    let record_id = command.defect_code.clone();
    let data = service(&state).create_defect_code(command).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_CREATE",
        "mdm.mdm_defect_codes",
        &record_id,
        data.clone(),
    )
    .await;
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_defect_code(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(defect_code): Path<String>,
    Json(command): Json<UpdateDefectCodeCommand>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state)
        .update_defect_code(&defect_code, command)
        .await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_UPDATE",
        "mdm.mdm_defect_codes",
        &defect_code,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn activate_defect_code(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(defect_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).activate_defect_code(&defect_code).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_ACTIVATE",
        "mdm.mdm_defect_codes",
        &defect_code,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn deactivate_defect_code(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(defect_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let data = service(&state).deactivate_defect_code(&defect_code).await?;
    audit_master_data_change(
        &state,
        &user,
        "MASTER_DATA_DEACTIVATE",
        "mdm.mdm_defect_codes",
        &defect_code,
        data.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(data)))
}
