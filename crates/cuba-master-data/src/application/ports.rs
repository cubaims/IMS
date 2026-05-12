use async_trait::async_trait;

use super::{
    BinCapacityUtilizationReadModel, BomComponentReadModel, BomDetailReadModel,
    BomExplosionPreviewReadModel, BomHeaderReadModel, BomSummaryReadModel, BomTreeReadModel,
    BomValidationReadModel, CopyBomCommand, CreateBomHeaderCommand, CreateCustomerCommand,
    CreateDefectCodeCommand, CreateInspectionCharCommand, CreateMaterialCommand,
    CreateMaterialSupplierCommand, CreateProductVariantCommand, CreateStorageBinCommand,
    CreateSupplierCommand, CreateWorkCenterCommand, CustomerReadModel, DefectCodeReadModel,
    DeleteAck, InspectionCharacteristicReadModel, MasterDataQuery, MaterialReadModel,
    MaterialSupplierReadModel, MutationAck, ProductVariantReadModel, StorageBinReadModel,
    SupplierReadModel, UpdateBomComponentCommand, UpdateBomHeaderCommand, UpdateCustomerCommand,
    UpdateDefectCodeCommand, UpdateInspectionCharCommand, UpdateMaterialCommand,
    UpdateMaterialSupplierCommand, UpdateProductVariantCommand, UpdateStorageBinCommand,
    UpdateSupplierCommand, UpdateWorkCenterCommand, WorkCenterReadModel,
};
use crate::domain::{Bom, BomId};
use cuba_shared::{AppResult, Page};

#[async_trait]
pub trait MaterialRepository: Send + Sync {
    async fn list_materials(&self, query: MasterDataQuery) -> AppResult<Page<MaterialReadModel>>;
    async fn get_material(&self, material_id: &str) -> AppResult<MaterialReadModel>;
    async fn create_material(&self, command: CreateMaterialCommand)
    -> AppResult<MaterialReadModel>;
    async fn update_material(
        &self,
        material_id: &str,
        command: UpdateMaterialCommand,
    ) -> AppResult<MaterialReadModel>;
    async fn activate_material(&self, material_id: &str) -> AppResult<MutationAck>;
    async fn deactivate_material(&self, material_id: &str) -> AppResult<MutationAck>;
}

#[async_trait]
pub trait StorageBinRepository: Send + Sync {
    async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Page<StorageBinReadModel>>;
    async fn get_bin(&self, bin_code: &str) -> AppResult<StorageBinReadModel>;
    async fn create_bin(&self, command: CreateStorageBinCommand) -> AppResult<StorageBinReadModel>;
    async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<StorageBinReadModel>;
    async fn activate_bin(&self, bin_code: &str) -> AppResult<MutationAck>;
    async fn deactivate_bin(&self, bin_code: &str) -> AppResult<MutationAck>;
    async fn get_bin_capacity_utilization(
        &self,
        bin_code: &str,
    ) -> AppResult<BinCapacityUtilizationReadModel>;
}

#[async_trait]
pub trait SupplierRepository: Send + Sync {
    async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Page<SupplierReadModel>>;
    async fn get_supplier(&self, supplier_id: &str) -> AppResult<SupplierReadModel>;
    async fn create_supplier(&self, command: CreateSupplierCommand)
    -> AppResult<SupplierReadModel>;
    async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<SupplierReadModel>;
    async fn activate_supplier(&self, supplier_id: &str) -> AppResult<MutationAck>;
    async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<MutationAck>;
}

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn list_customers(&self, query: MasterDataQuery) -> AppResult<Page<CustomerReadModel>>;
    async fn get_customer(&self, customer_id: &str) -> AppResult<CustomerReadModel>;
    async fn create_customer(&self, command: CreateCustomerCommand)
    -> AppResult<CustomerReadModel>;
    async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<CustomerReadModel>;
    async fn activate_customer(&self, customer_id: &str) -> AppResult<MutationAck>;
    async fn deactivate_customer(&self, customer_id: &str) -> AppResult<MutationAck>;
}

#[async_trait]
pub trait MaterialSupplierRepository: Send + Sync {
    async fn list_material_suppliers(
        &self,
        material_id: &str,
    ) -> AppResult<Vec<MaterialSupplierReadModel>>;
    async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<MaterialSupplierReadModel>;
    async fn update_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
        command: UpdateMaterialSupplierCommand,
    ) -> AppResult<MaterialSupplierReadModel>;
    async fn set_primary_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<MaterialSupplierReadModel>;
    async fn cancel_primary_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<MaterialSupplierReadModel>;
    async fn remove_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<DeleteAck>;
}

#[async_trait]
pub trait ProductVariantRepository: Send + Sync {
    async fn list_variants(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<ProductVariantReadModel>>;
    async fn get_variant(&self, variant_code: &str) -> AppResult<ProductVariantReadModel>;
    async fn create_variant(
        &self,
        command: CreateProductVariantCommand,
    ) -> AppResult<ProductVariantReadModel>;
    async fn update_variant(
        &self,
        variant_code: &str,
        command: UpdateProductVariantCommand,
    ) -> AppResult<ProductVariantReadModel>;
    async fn activate_variant(&self, variant_code: &str) -> AppResult<MutationAck>;
    async fn deactivate_variant(&self, variant_code: &str) -> AppResult<MutationAck>;
}

#[async_trait]
pub trait BomRepository: Send + Sync {
    async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Page<BomSummaryReadModel>>;
    async fn get_bom(&self, bom_id: &str) -> AppResult<BomDetailReadModel>;
    async fn create_bom(&self, command: CreateBomHeaderCommand) -> AppResult<BomHeaderReadModel>;
    async fn copy_bom(&self, bom: &Bom, command: CopyBomCommand) -> AppResult<BomDetailReadModel>;
    async fn update_bom(
        &self,
        bom_id: &str,
        command: UpdateBomHeaderCommand,
    ) -> AppResult<BomHeaderReadModel>;

    async fn list_components(&self, bom_id: &str) -> AppResult<Vec<BomComponentReadModel>>;
    async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<BomComponentReadModel>;
    async fn update_component_for_bom(
        &self,
        bom_id: &str,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<BomComponentReadModel>;
    async fn remove_component(&self, component_id: i64) -> AppResult<DeleteAck>;
    async fn remove_component_for_bom(
        &self,
        bom_id: &str,
        component_id: i64,
    ) -> AppResult<DeleteAck>;

    async fn get_bom_tree(&self, bom_id: &str) -> AppResult<BomTreeReadModel>;
    async fn validate_bom(&self, bom_id: &str) -> AppResult<BomValidationReadModel>;
    async fn preview_bom_explosion(
        &self,
        material_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<BomExplosionPreviewReadModel>;

    // ============================================================
    // 聚合路径(Phase 2 新增,与上面的逐字段 API 并存)
    // ============================================================

    /// 加载完整的 BOM 聚合(header + components 一次性拼回)。
    /// BOM 不存在时返回 `AppError::Business { code: "BOM_NOT_FOUND", .. }`。
    async fn load_bom(&self, bom_id: &BomId) -> AppResult<Bom>;

    /// 持久化聚合的当前状态(基于 (parent, component) 边的 diff)。
    /// 写入在单个事务内完成。
    async fn save_bom(&self, bom: &Bom) -> AppResult<()>;

    /// 跨聚合不变式:把 `bom` 的当前组件列表与 DB 中其他 active BOM 的边
    /// 拼成全局图,跑一次循环检测。检测到环时返回
    /// `AppError::Business { code: "BOM_CYCLE_DETECTED", .. }`。
    ///
    /// 注:无论 `bom.header().status` 当前是 Draft 还是 Active,这里都把它的
    /// 边纳入检测——基于"草稿迟早会激活"的保守原则,跟现有
    /// `activate_bom` 里的语义一致。
    async fn assert_no_cycle_after_change(&self, bom: &Bom) -> AppResult<()>;
}

#[async_trait]
pub trait WorkCenterRepository: Send + Sync {
    async fn list_work_centers(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<WorkCenterReadModel>>;
    async fn get_work_center(&self, work_center_id: &str) -> AppResult<WorkCenterReadModel>;
    async fn create_work_center(
        &self,
        command: CreateWorkCenterCommand,
    ) -> AppResult<WorkCenterReadModel>;
    async fn update_work_center(
        &self,
        work_center_id: &str,
        command: UpdateWorkCenterCommand,
    ) -> AppResult<WorkCenterReadModel>;
    async fn activate_work_center(&self, work_center_id: &str) -> AppResult<MutationAck>;
    async fn deactivate_work_center(&self, work_center_id: &str) -> AppResult<MutationAck>;
}

#[async_trait]
pub trait QualityMasterRepository: Send + Sync {
    async fn list_inspection_chars(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<InspectionCharacteristicReadModel>>;
    async fn get_inspection_char(
        &self,
        char_id: &str,
    ) -> AppResult<InspectionCharacteristicReadModel>;
    async fn create_inspection_char(
        &self,
        command: CreateInspectionCharCommand,
    ) -> AppResult<InspectionCharacteristicReadModel>;
    async fn update_inspection_char(
        &self,
        char_id: &str,
        command: UpdateInspectionCharCommand,
    ) -> AppResult<InspectionCharacteristicReadModel>;
    async fn activate_inspection_char(&self, char_id: &str) -> AppResult<MutationAck>;
    async fn deactivate_inspection_char(&self, char_id: &str) -> AppResult<MutationAck>;

    async fn list_defect_codes(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<DefectCodeReadModel>>;
    async fn get_defect_code(&self, defect_code: &str) -> AppResult<DefectCodeReadModel>;
    async fn create_defect_code(
        &self,
        command: CreateDefectCodeCommand,
    ) -> AppResult<DefectCodeReadModel>;
    async fn update_defect_code(
        &self,
        defect_code: &str,
        command: UpdateDefectCodeCommand,
    ) -> AppResult<DefectCodeReadModel>;
    async fn activate_defect_code(&self, defect_code: &str) -> AppResult<MutationAck>;
    async fn deactivate_defect_code(&self, defect_code: &str) -> AppResult<MutationAck>;
}
