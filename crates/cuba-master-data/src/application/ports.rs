use async_trait::async_trait;
use serde_json::Value;

use cuba_shared::AppResult;

use super::{
    CreateBomComponentCommand, CreateBomHeaderCommand, CreateCustomerCommand,
    CreateDefectCodeCommand, CreateInspectionCharCommand, CreateMaterialCommand,
    CreateMaterialSupplierCommand, CreateProductVariantCommand, CreateStorageBinCommand,
    CreateSupplierCommand, CreateWorkCenterCommand, MasterDataQuery, UpdateBomComponentCommand,
    UpdateBomHeaderCommand, UpdateCustomerCommand, UpdateDefectCodeCommand,
    UpdateInspectionCharCommand, UpdateMaterialCommand, UpdateMaterialSupplierCommand,
    UpdateProductVariantCommand, UpdateStorageBinCommand, UpdateSupplierCommand,
    UpdateWorkCenterCommand,
};

#[async_trait]
pub trait MaterialRepository: Send + Sync {
    async fn list_materials(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_material(&self, material_id: &str) -> AppResult<Value>;
    async fn create_material(&self, command: CreateMaterialCommand) -> AppResult<Value>;
    async fn update_material(
        &self,
        material_id: &str,
        command: UpdateMaterialCommand,
    ) -> AppResult<Value>;
    async fn activate_material(&self, material_id: &str) -> AppResult<Value>;
    async fn deactivate_material(&self, material_id: &str) -> AppResult<Value>;
}

#[async_trait]
pub trait StorageBinRepository: Send + Sync {
    async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_bin(&self, bin_code: &str) -> AppResult<Value>;
    async fn create_bin(&self, command: CreateStorageBinCommand) -> AppResult<Value>;
    async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<Value>;
    async fn activate_bin(&self, bin_code: &str) -> AppResult<Value>;
    async fn deactivate_bin(&self, bin_code: &str) -> AppResult<Value>;
}

#[async_trait]
pub trait SupplierRepository: Send + Sync {
    async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_supplier(&self, supplier_id: &str) -> AppResult<Value>;
    async fn create_supplier(&self, command: CreateSupplierCommand) -> AppResult<Value>;
    async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<Value>;
    async fn activate_supplier(&self, supplier_id: &str) -> AppResult<Value>;
    async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<Value>;
}

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn list_customers(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_customer(&self, customer_id: &str) -> AppResult<Value>;
    async fn create_customer(&self, command: CreateCustomerCommand) -> AppResult<Value>;
    async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<Value>;
    async fn activate_customer(&self, customer_id: &str) -> AppResult<Value>;
    async fn deactivate_customer(&self, customer_id: &str) -> AppResult<Value>;
}

#[async_trait]
pub trait MaterialSupplierRepository: Send + Sync {
    async fn list_material_suppliers(&self, material_id: &str) -> AppResult<Value>;
    async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<Value>;
    async fn update_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
        command: UpdateMaterialSupplierCommand,
    ) -> AppResult<Value>;
    async fn set_primary_supplier(&self, material_id: &str, supplier_id: &str) -> AppResult<Value>;
    async fn remove_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<Value>;
}

#[async_trait]
pub trait ProductVariantRepository: Send + Sync {
    async fn list_variants(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_variant(&self, variant_code: &str) -> AppResult<Value>;
    async fn create_variant(&self, command: CreateProductVariantCommand) -> AppResult<Value>;
    async fn update_variant(
        &self,
        variant_code: &str,
        command: UpdateProductVariantCommand,
    ) -> AppResult<Value>;
    async fn activate_variant(&self, variant_code: &str) -> AppResult<Value>;
    async fn deactivate_variant(&self, variant_code: &str) -> AppResult<Value>;
}

#[async_trait]
pub trait BomRepository: Send + Sync {
    async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_bom(&self, bom_id: &str) -> AppResult<Value>;
    async fn create_bom(&self, command: CreateBomHeaderCommand) -> AppResult<Value>;
    async fn update_bom(&self, bom_id: &str, command: UpdateBomHeaderCommand) -> AppResult<Value>;
    async fn activate_bom(&self, bom_id: &str) -> AppResult<Value>;
    async fn deactivate_bom(&self, bom_id: &str) -> AppResult<Value>;

    async fn list_components(&self, bom_id: &str) -> AppResult<Value>;
    async fn add_component(&self, command: CreateBomComponentCommand) -> AppResult<Value>;
    async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<Value>;
    async fn remove_component(&self, component_id: i64) -> AppResult<Value>;

    async fn get_bom_tree(&self, bom_id: &str) -> AppResult<Value>;
    async fn validate_bom(&self, bom_id: &str) -> AppResult<Value>;
    async fn preview_bom_explosion(
        &self,
        material_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<Value>;
}

#[async_trait]
pub trait WorkCenterRepository: Send + Sync {
    async fn list_work_centers(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_work_center(&self, work_center_id: &str) -> AppResult<Value>;
    async fn create_work_center(&self, command: CreateWorkCenterCommand) -> AppResult<Value>;
    async fn update_work_center(
        &self,
        work_center_id: &str,
        command: UpdateWorkCenterCommand,
    ) -> AppResult<Value>;
    async fn activate_work_center(&self, work_center_id: &str) -> AppResult<Value>;
    async fn deactivate_work_center(&self, work_center_id: &str) -> AppResult<Value>;
}

#[async_trait]
pub trait QualityMasterRepository: Send + Sync {
    async fn list_inspection_chars(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_inspection_char(&self, char_id: &str) -> AppResult<Value>;
    async fn create_inspection_char(
        &self,
        command: CreateInspectionCharCommand,
    ) -> AppResult<Value>;
    async fn update_inspection_char(
        &self,
        char_id: &str,
        command: UpdateInspectionCharCommand,
    ) -> AppResult<Value>;

    async fn list_defect_codes(&self, query: MasterDataQuery) -> AppResult<Value>;
    async fn get_defect_code(&self, defect_code: &str) -> AppResult<Value>;
    async fn create_defect_code(&self, command: CreateDefectCodeCommand) -> AppResult<Value>;
    async fn update_defect_code(
        &self,
        defect_code: &str,
        command: UpdateDefectCodeCommand,
    ) -> AppResult<Value>;
    async fn activate_defect_code(&self, defect_code: &str) -> AppResult<Value>;
    async fn deactivate_defect_code(&self, defect_code: &str) -> AppResult<Value>;
}
