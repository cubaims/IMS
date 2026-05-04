use std::sync::Arc;

use serde_json::Value;
use validator::Validate;

use cuba_shared::{AppError, AppResult};

use super::{
    BomRepository, CreateBomComponentCommand, CreateBomHeaderCommand, CreateCustomerCommand,
    CreateDefectCodeCommand, CreateInspectionCharCommand, CreateMaterialCommand,
    CreateMaterialSupplierCommand, CreateProductVariantCommand, CreateStorageBinCommand,
    CreateSupplierCommand, CreateWorkCenterCommand, CustomerRepository, MasterDataQuery,
    MaterialRepository, MaterialSupplierRepository, ProductVariantRepository,
    QualityMasterRepository, StorageBinRepository, SupplierRepository, UpdateBomComponentCommand,
    UpdateBomHeaderCommand, UpdateCustomerCommand, UpdateDefectCodeCommand,
    UpdateInspectionCharCommand, UpdateMaterialCommand, UpdateMaterialSupplierCommand,
    UpdateProductVariantCommand, UpdateStorageBinCommand, UpdateSupplierCommand,
    UpdateWorkCenterCommand, WorkCenterRepository,
};

#[derive(Clone)]
pub struct MasterDataService {
    material_repo: Arc<dyn MaterialRepository>,
    bin_repo: Arc<dyn StorageBinRepository>,
    supplier_repo: Arc<dyn SupplierRepository>,
    customer_repo: Arc<dyn CustomerRepository>,
    material_supplier_repo: Arc<dyn MaterialSupplierRepository>,
    variant_repo: Arc<dyn ProductVariantRepository>,
    bom_repo: Arc<dyn BomRepository>,
    work_center_repo: Arc<dyn WorkCenterRepository>,
    quality_master_repo: Arc<dyn QualityMasterRepository>,
}

impl MasterDataService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        material_repo: Arc<dyn MaterialRepository>,
        bin_repo: Arc<dyn StorageBinRepository>,
        supplier_repo: Arc<dyn SupplierRepository>,
        customer_repo: Arc<dyn CustomerRepository>,
        material_supplier_repo: Arc<dyn MaterialSupplierRepository>,
        variant_repo: Arc<dyn ProductVariantRepository>,
        bom_repo: Arc<dyn BomRepository>,
        work_center_repo: Arc<dyn WorkCenterRepository>,
        quality_master_repo: Arc<dyn QualityMasterRepository>,
    ) -> Self {
        Self {
            material_repo,
            bin_repo,
            supplier_repo,
            customer_repo,
            material_supplier_repo,
            variant_repo,
            bom_repo,
            work_center_repo,
            quality_master_repo,
        }
    }

    fn validate<T: Validate>(command: &T) -> AppResult<()> {
        command
            .validate()
            .map_err(|error| AppError::Validation(error.to_string()))
    }

    pub async fn list_materials(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.material_repo.list_materials(query).await
    }

    pub async fn get_material(&self, material_id: &str) -> AppResult<Value> {
        self.material_repo.get_material(material_id).await
    }

    pub async fn create_material(&self, command: CreateMaterialCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.material_repo.create_material(command).await
    }

    pub async fn update_material(
        &self,
        material_id: &str,
        command: UpdateMaterialCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.material_repo
            .update_material(material_id, command)
            .await
    }

    pub async fn activate_material(&self, material_id: &str) -> AppResult<Value> {
        self.material_repo.activate_material(material_id).await
    }

    pub async fn deactivate_material(&self, material_id: &str) -> AppResult<Value> {
        self.material_repo.deactivate_material(material_id).await
    }

    pub async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.bin_repo.list_bins(query).await
    }

    pub async fn get_bin(&self, bin_code: &str) -> AppResult<Value> {
        self.bin_repo.get_bin(bin_code).await
    }

    pub async fn create_bin(&self, command: CreateStorageBinCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.bin_repo.create_bin(command).await
    }

    pub async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.bin_repo.update_bin(bin_code, command).await
    }

    pub async fn activate_bin(&self, bin_code: &str) -> AppResult<Value> {
        self.bin_repo.activate_bin(bin_code).await
    }

    pub async fn deactivate_bin(&self, bin_code: &str) -> AppResult<Value> {
        self.bin_repo.deactivate_bin(bin_code).await
    }

    pub async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.supplier_repo.list_suppliers(query).await
    }

    pub async fn get_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        self.supplier_repo.get_supplier(supplier_id).await
    }

    pub async fn create_supplier(&self, command: CreateSupplierCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.supplier_repo.create_supplier(command).await
    }

    pub async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.supplier_repo
            .update_supplier(supplier_id, command)
            .await
    }

    pub async fn activate_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        self.supplier_repo.activate_supplier(supplier_id).await
    }

    pub async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        self.supplier_repo.deactivate_supplier(supplier_id).await
    }

    pub async fn list_customers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.customer_repo.list_customers(query).await
    }

    pub async fn get_customer(&self, customer_id: &str) -> AppResult<Value> {
        self.customer_repo.get_customer(customer_id).await
    }

    pub async fn create_customer(&self, command: CreateCustomerCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.customer_repo.create_customer(command).await
    }

    pub async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.customer_repo
            .update_customer(customer_id, command)
            .await
    }

    pub async fn activate_customer(&self, customer_id: &str) -> AppResult<Value> {
        self.customer_repo.activate_customer(customer_id).await
    }

    pub async fn deactivate_customer(&self, customer_id: &str) -> AppResult<Value> {
        self.customer_repo.deactivate_customer(customer_id).await
    }

    pub async fn list_material_suppliers(&self, material_id: &str) -> AppResult<Value> {
        self.material_supplier_repo
            .list_material_suppliers(material_id)
            .await
    }

    pub async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.material_supplier_repo
            .create_material_supplier(command)
            .await
    }

    pub async fn update_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
        command: UpdateMaterialSupplierCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.material_supplier_repo
            .update_material_supplier(material_id, supplier_id, command)
            .await
    }

    pub async fn set_primary_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<Value> {
        self.material_supplier_repo
            .set_primary_supplier(material_id, supplier_id)
            .await
    }

    pub async fn remove_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<Value> {
        self.material_supplier_repo
            .remove_material_supplier(material_id, supplier_id)
            .await
    }

    pub async fn list_variants(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.variant_repo.list_variants(query).await
    }

    pub async fn get_variant(&self, variant_code: &str) -> AppResult<Value> {
        self.variant_repo.get_variant(variant_code).await
    }

    pub async fn create_variant(&self, command: CreateProductVariantCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.variant_repo.create_variant(command).await
    }

    pub async fn update_variant(
        &self,
        variant_code: &str,
        command: UpdateProductVariantCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.variant_repo
            .update_variant(variant_code, command)
            .await
    }

    pub async fn activate_variant(&self, variant_code: &str) -> AppResult<Value> {
        self.variant_repo.activate_variant(variant_code).await
    }

    pub async fn deactivate_variant(&self, variant_code: &str) -> AppResult<Value> {
        self.variant_repo.deactivate_variant(variant_code).await
    }

    pub async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.bom_repo.list_boms(query).await
    }

    pub async fn get_bom(&self, bom_id: &str) -> AppResult<Value> {
        self.bom_repo.get_bom(bom_id).await
    }

    pub async fn create_bom(&self, command: CreateBomHeaderCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.bom_repo.create_bom(command).await
    }

    pub async fn update_bom(
        &self,
        bom_id: &str,
        command: UpdateBomHeaderCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.bom_repo.update_bom(bom_id, command).await
    }

    pub async fn activate_bom(&self, bom_id: &str) -> AppResult<Value> {
        self.bom_repo.activate_bom(bom_id).await
    }

    pub async fn deactivate_bom(&self, bom_id: &str) -> AppResult<Value> {
        self.bom_repo.deactivate_bom(bom_id).await
    }

    pub async fn list_components(&self, bom_id: &str) -> AppResult<Value> {
        self.bom_repo.list_components(bom_id).await
    }

    pub async fn add_component(&self, command: CreateBomComponentCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.bom_repo.add_component(command).await
    }

    pub async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.bom_repo.update_component(component_id, command).await
    }

    pub async fn remove_component(&self, component_id: i64) -> AppResult<Value> {
        self.bom_repo.remove_component(component_id).await
    }

    pub async fn get_bom_tree(&self, bom_id: &str) -> AppResult<Value> {
        self.bom_repo.get_bom_tree(bom_id).await
    }

    pub async fn validate_bom(&self, bom_id: &str) -> AppResult<Value> {
        self.bom_repo.validate_bom(bom_id).await
    }

    pub async fn preview_bom_explosion(
        &self,
        material_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<Value> {
        self.bom_repo
            .preview_bom_explosion(material_id, quantity, variant_code)
            .await
    }

    pub async fn list_work_centers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.work_center_repo.list_work_centers(query).await
    }

    pub async fn get_work_center(&self, work_center_id: &str) -> AppResult<Value> {
        self.work_center_repo.get_work_center(work_center_id).await
    }

    pub async fn create_work_center(&self, command: CreateWorkCenterCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.work_center_repo.create_work_center(command).await
    }

    pub async fn update_work_center(
        &self,
        work_center_id: &str,
        command: UpdateWorkCenterCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.work_center_repo
            .update_work_center(work_center_id, command)
            .await
    }

    pub async fn activate_work_center(&self, work_center_id: &str) -> AppResult<Value> {
        self.work_center_repo
            .activate_work_center(work_center_id)
            .await
    }

    pub async fn deactivate_work_center(&self, work_center_id: &str) -> AppResult<Value> {
        self.work_center_repo
            .deactivate_work_center(work_center_id)
            .await
    }

    pub async fn list_inspection_chars(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.quality_master_repo.list_inspection_chars(query).await
    }

    pub async fn get_inspection_char(&self, char_id: &str) -> AppResult<Value> {
        self.quality_master_repo.get_inspection_char(char_id).await
    }

    pub async fn create_inspection_char(
        &self,
        command: CreateInspectionCharCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.quality_master_repo
            .create_inspection_char(command)
            .await
    }

    pub async fn update_inspection_char(
        &self,
        char_id: &str,
        command: UpdateInspectionCharCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.quality_master_repo
            .update_inspection_char(char_id, command)
            .await
    }

    pub async fn list_defect_codes(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.quality_master_repo.list_defect_codes(query).await
    }

    pub async fn get_defect_code(&self, defect_code: &str) -> AppResult<Value> {
        self.quality_master_repo.get_defect_code(defect_code).await
    }

    pub async fn create_defect_code(&self, command: CreateDefectCodeCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        self.quality_master_repo.create_defect_code(command).await
    }

    pub async fn update_defect_code(
        &self,
        defect_code: &str,
        command: UpdateDefectCodeCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        self.quality_master_repo
            .update_defect_code(defect_code, command)
            .await
    }

    pub async fn activate_defect_code(&self, defect_code: &str) -> AppResult<Value> {
        self.quality_master_repo
            .activate_defect_code(defect_code)
            .await
    }

    pub async fn deactivate_defect_code(&self, defect_code: &str) -> AppResult<Value> {
        self.quality_master_repo
            .deactivate_defect_code(defect_code)
            .await
    }
}
