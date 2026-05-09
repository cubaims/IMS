use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use cuba_shared::AppState;

use super::handlers;

/// 只读路由 = 全部 GET。
///
/// 调用方(cuba-api)在外层挂 `master-data:read` 权限时只关心这一份。
pub fn read_routes() -> Router<AppState> {
    Router::new()
        // materials
        .route("/materials", get(handlers::list_materials))
        .route("/materials/{material_id}", get(handlers::get_material))
        .route(
            "/materials/{material_id}/suppliers",
            get(handlers::list_material_suppliers),
        )
        // bins
        .route("/bins", get(handlers::list_bins))
        .route("/bins/{bin_code}", get(handlers::get_bin))
        .route(
            "/bins/{bin_code}/capacity-utilization",
            get(handlers::get_bin_capacity_utilization),
        )
        // suppliers
        .route("/suppliers", get(handlers::list_suppliers))
        .route("/suppliers/{supplier_id}", get(handlers::get_supplier))
        // customers
        .route("/customers", get(handlers::list_customers))
        .route("/customers/{customer_id}", get(handlers::get_customer))
        // product variants
        .route("/product-variants", get(handlers::list_variants))
        .route(
            "/product-variants/{variant_code}",
            get(handlers::get_variant),
        )
        // boms
        .route("/boms", get(handlers::list_boms))
        .route(
            "/boms/explode-preview",
            get(handlers::preview_bom_explosion),
        )
        .route("/boms/{bom_id}", get(handlers::get_bom))
        .route(
            "/boms/{bom_id}/components",
            get(handlers::list_bom_components),
        )
        .route("/boms/{bom_id}/tree", get(handlers::get_bom_tree))
        // work centers
        .route("/work-centers", get(handlers::list_work_centers))
        .route(
            "/work-centers/{work_center_id}",
            get(handlers::get_work_center),
        )
        // inspection characteristics
        .route("/inspection-chars", get(handlers::list_inspection_chars))
        .route(
            "/inspection-chars/{char_id}",
            get(handlers::get_inspection_char),
        )
        // defect codes
        .route("/defect-codes", get(handlers::list_defect_codes))
        .route(
            "/defect-codes/{defect_code}",
            get(handlers::get_defect_code),
        )
}

/// 写路由 = POST + PATCH + DELETE。
///
/// 含创建、修改、启用/停用、关联设置、校验等所有变更动作。
/// 注意:`/boms/{bom_id}/validate` 走 POST,虽然是只读校验,
/// 但路由是 POST、且通常需要写权限,所以归到写路由里。
pub fn write_routes() -> Router<AppState> {
    Router::new()
        // materials
        .route("/materials", post(handlers::create_material))
        .route("/materials/{material_id}", patch(handlers::update_material))
        .route(
            "/materials/{material_id}/activate",
            post(handlers::activate_material),
        )
        .route(
            "/materials/{material_id}/deactivate",
            post(handlers::deactivate_material),
        )
        .route(
            "/materials/{material_id}/suppliers",
            post(handlers::create_material_supplier),
        )
        .route(
            "/materials/{material_id}/suppliers/{supplier_id}",
            patch(handlers::update_material_supplier),
        )
        .route(
            "/materials/{material_id}/suppliers/{supplier_id}",
            delete(handlers::remove_material_supplier),
        )
        .route(
            "/materials/{material_id}/suppliers/{supplier_id}/primary",
            post(handlers::set_primary_supplier),
        )
        // bins
        .route("/bins", post(handlers::create_bin))
        .route("/bins/{bin_code}", patch(handlers::update_bin))
        .route("/bins/{bin_code}/activate", post(handlers::activate_bin))
        .route(
            "/bins/{bin_code}/deactivate",
            post(handlers::deactivate_bin),
        )
        // suppliers
        .route("/suppliers", post(handlers::create_supplier))
        .route("/suppliers/{supplier_id}", patch(handlers::update_supplier))
        .route(
            "/suppliers/{supplier_id}/activate",
            post(handlers::activate_supplier),
        )
        .route(
            "/suppliers/{supplier_id}/deactivate",
            post(handlers::deactivate_supplier),
        )
        // customers
        .route("/customers", post(handlers::create_customer))
        .route("/customers/{customer_id}", patch(handlers::update_customer))
        .route(
            "/customers/{customer_id}/activate",
            post(handlers::activate_customer),
        )
        .route(
            "/customers/{customer_id}/deactivate",
            post(handlers::deactivate_customer),
        )
        // product variants
        .route("/product-variants", post(handlers::create_variant))
        .route(
            "/product-variants/{variant_code}",
            patch(handlers::update_variant),
        )
        .route(
            "/product-variants/{variant_code}/activate",
            post(handlers::activate_variant),
        )
        .route(
            "/product-variants/{variant_code}/deactivate",
            post(handlers::deactivate_variant),
        )
        // boms
        .route("/boms", post(handlers::create_bom))
        .route("/boms/{bom_id}", patch(handlers::update_bom))
        .route("/boms/{bom_id}/activate", post(handlers::activate_bom))
        .route("/boms/{bom_id}/deactivate", post(handlers::deactivate_bom))
        .route(
            "/boms/{bom_id}/components",
            post(handlers::add_bom_component),
        )
        .route(
            "/boms/components/{component_id}",
            patch(handlers::update_bom_component),
        )
        .route(
            "/boms/components/{component_id}",
            delete(handlers::remove_bom_component),
        )
        .route("/boms/{bom_id}/validate", post(handlers::validate_bom))
        // work centers
        .route("/work-centers", post(handlers::create_work_center))
        .route(
            "/work-centers/{work_center_id}",
            patch(handlers::update_work_center),
        )
        .route(
            "/work-centers/{work_center_id}/activate",
            post(handlers::activate_work_center),
        )
        .route(
            "/work-centers/{work_center_id}/deactivate",
            post(handlers::deactivate_work_center),
        )
        // inspection characteristics
        .route("/inspection-chars", post(handlers::create_inspection_char))
        .route(
            "/inspection-chars/{char_id}",
            patch(handlers::update_inspection_char),
        )
        // defect codes
        .route("/defect-codes", post(handlers::create_defect_code))
        .route(
            "/defect-codes/{defect_code}",
            patch(handlers::update_defect_code),
        )
        .route(
            "/defect-codes/{defect_code}/activate",
            post(handlers::activate_defect_code),
        )
        .route(
            "/defect-codes/{defect_code}/deactivate",
            post(handlers::deactivate_defect_code),
        )
}

/// 向后兼容:把 read 和 write 合在一起。
/// 新代码应改用 `read_routes()` / `write_routes()` 由 cuba-api 分别挂权限。
pub fn routes() -> Router<AppState> {
    read_routes().merge(write_routes())
}
