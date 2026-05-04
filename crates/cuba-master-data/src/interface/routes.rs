use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use cuba_shared::AppState;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        // materials
        .route("/materials", get(handlers::list_materials))
        .route("/materials", post(handlers::create_material))
        .route("/materials/{material_id}", get(handlers::get_material))
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
            get(handlers::list_material_suppliers),
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
        .route("/bins", get(handlers::list_bins))
        .route("/bins", post(handlers::create_bin))
        .route("/bins/{bin_code}", get(handlers::get_bin))
        .route("/bins/{bin_code}", patch(handlers::update_bin))
        .route("/bins/{bin_code}/activate", post(handlers::activate_bin))
        .route(
            "/bins/{bin_code}/deactivate",
            post(handlers::deactivate_bin),
        )
        // suppliers
        .route("/suppliers", get(handlers::list_suppliers))
        .route("/suppliers", post(handlers::create_supplier))
        .route("/suppliers/{supplier_id}", get(handlers::get_supplier))
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
        .route("/customers", get(handlers::list_customers))
        .route("/customers", post(handlers::create_customer))
        .route("/customers/{customer_id}", get(handlers::get_customer))
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
        .route("/product-variants", get(handlers::list_variants))
        .route("/product-variants", post(handlers::create_variant))
        .route(
            "/product-variants/{variant_code}",
            get(handlers::get_variant),
        )
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
        .route("/boms", get(handlers::list_boms))
        .route("/boms", post(handlers::create_bom))
        .route("/boms/explode-preview", get(handlers::preview_bom_explosion))
        .route("/boms/{bom_id}", get(handlers::get_bom))
        .route("/boms/{bom_id}", patch(handlers::update_bom))
        .route("/boms/{bom_id}/activate", post(handlers::activate_bom))
        .route(
            "/boms/{bom_id}/deactivate",
            post(handlers::deactivate_bom),
        )
        .route(
            "/boms/{bom_id}/components",
            get(handlers::list_bom_components),
        )
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
        .route("/boms/{bom_id}/tree", get(handlers::get_bom_tree))
        .route("/boms/{bom_id}/validate", post(handlers::validate_bom))
        // work centers
        .route("/work-centers", get(handlers::list_work_centers))
        .route("/work-centers", post(handlers::create_work_center))
        .route(
            "/work-centers/{work_center_id}",
            get(handlers::get_work_center),
        )
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
        .route(
            "/inspection-chars",
            get(handlers::list_inspection_chars),
        )
        .route(
            "/inspection-chars",
            post(handlers::create_inspection_char),
        )
        .route(
            "/inspection-chars/{char_id}",
            get(handlers::get_inspection_char),
        )
        .route(
            "/inspection-chars/{char_id}",
            patch(handlers::update_inspection_char),
        )
        // defect codes
        .route("/defect-codes", get(handlers::list_defect_codes))
        .route("/defect-codes", post(handlers::create_defect_code))
        .route(
            "/defect-codes/{defect_code}",
            get(handlers::get_defect_code),
        )
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