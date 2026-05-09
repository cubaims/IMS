use std::sync::Arc;

use serde_json::Value;
use validator::Validate;

use cuba_shared::{AppError, AppResult};
use crate::domain::{BinCode, BomComponent, BomHeader, BomId, Customer, CustomerId, Material, MaterialId, MaterialSupplier, MaterialType, ProductVariant, StorageBin, Supplier, SupplierId, VariantCode};
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
        // 计划 §五.1 领域规则:物料编码非空 + 长度限制、名称非空、base_unit 非空、
        // 安全库存/标准成本/MAP >= 0。这些都在 entity 构造里强制。
        let material_id = MaterialId::new(command.material_id.clone())?;
        let material_type =
            MaterialType::from_db_value(&command.material_type).ok_or_else(|| {
                AppError::Validation(format!(
                    "未知的物料类型 '{}',应为 原材料/半成品/成品",
                    command.material_type
                ))
            })?;
        let _entity = Material::new(
            material_id,
            &command.material_name,
            material_type,
            &command.base_unit,
            &command.default_zone,
            command.safety_stock,
            command.reorder_point,
            command.standard_price,
            command.map_price,
        )?;

        self.material_repo.create_material(command).await
    }

    pub async fn update_material(
        &self,
        material_id: &str,
        command: UpdateMaterialCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        let _id = MaterialId::new(material_id)?;
        self.material_repo
            .update_material(material_id, command)
            .await
    }

    pub async fn activate_material(&self, material_id: &str) -> AppResult<Value> {
        let _id = MaterialId::new(material_id)?;
        self.material_repo.activate_material(material_id).await
    }

    pub async fn deactivate_material(&self, material_id: &str) -> AppResult<Value> {
        let _id = MaterialId::new(material_id)?;
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
        // 计划 §五.2 领域规则:bin_code 非空+长度限制、zone/bin_type 非空、capacity >= 0
        let bin_code = BinCode::new(command.bin_code.clone())?;
        let _entity = StorageBin::new(bin_code, &command.zone, &command.bin_type, command.capacity)?;

        self.bin_repo.create_bin(command).await
    }

    pub async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        // BinCode::new 在这里只做 ID 格式校验;capacity 跨字段规则
        // (capacity >= current_occupied)在 postgres.rs::update_bin 里
        // 用 StorageBin::change_capacity 跑,因为只在那一层能拿到 current_occupied。
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.update_bin(bin_code, command).await
    }

    pub async fn activate_bin(&self, bin_code: &str) -> AppResult<Value> {
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.activate_bin(bin_code).await
    }

    pub async fn deactivate_bin(&self, bin_code: &str) -> AppResult<Value> {
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.deactivate_bin(bin_code).await
    }

    /// 计划 §五.2:查询货位容量利用率
    pub async fn get_bin_capacity_utilization(&self, bin_code: &str) -> AppResult<Value> {
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.get_bin_capacity_utilization(bin_code).await
    }

    pub async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.supplier_repo.list_suppliers(query).await
    }

    pub async fn get_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        self.supplier_repo.get_supplier(supplier_id).await
    }

    pub async fn create_supplier(&self, command: CreateSupplierCommand) -> AppResult<Value> {
        Self::validate(&command)?;
        // 计划 §五.3 领域规则:供应商编码非空+长度限制、供应商名称非空
        let supplier_id = SupplierId::new(command.supplier_id.clone())?;
        let _entity = Supplier::new(supplier_id, &command.supplier_name)?;

        self.supplier_repo.create_supplier(command).await
    }

    pub async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        let _id = SupplierId::new(supplier_id)?;
        self.supplier_repo
            .update_supplier(supplier_id, command)
            .await
    }

    pub async fn activate_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        let _id = SupplierId::new(supplier_id)?;
        self.supplier_repo.activate_supplier(supplier_id).await
    }

    pub async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        let _id = SupplierId::new(supplier_id)?;
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
        // 计划 §五.5 领域规则:客户编码非空+长度限制、客户名称非空
        let customer_id = CustomerId::new(command.customer_id.clone())?;
        let _entity = Customer::new(customer_id, &command.customer_name)?;

        self.customer_repo.create_customer(command).await
    }

    pub async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        let _id = CustomerId::new(customer_id)?;
        self.customer_repo
            .update_customer(customer_id, command)
            .await
    }

    pub async fn activate_customer(&self, customer_id: &str) -> AppResult<Value> {
        let _id = CustomerId::new(customer_id)?;
        self.customer_repo.activate_customer(customer_id).await
    }

    pub async fn deactivate_customer(&self, customer_id: &str) -> AppResult<Value> {
        let _id = CustomerId::new(customer_id)?;
        self.customer_repo.deactivate_customer(customer_id).await
    }

    pub async fn list_material_suppliers(&self, material_id: &str) -> AppResult<Value> {
        let _id = MaterialId::new(material_id)?;
        self.material_supplier_repo
            .list_material_suppliers(material_id)
            .await
    }

    pub async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        // 计划 §五.4 领域规则:采购提前期 >= 0、最小采购量 >= 0。
        let material_id = MaterialId::new(command.material_id.clone())?;
        let supplier_id = SupplierId::new(command.supplier_id.clone())?;
        let _entity = MaterialSupplier::new(
            material_id,
            supplier_id,
            command.is_primary.unwrap_or(false),
            command.lead_time_days.unwrap_or(0),
            command.moq.unwrap_or(1),
        )?;
        // 跨表规则(物料/供应商必须 active)在 postgres.rs::create_material_supplier
        // 内事务里检查;放在 service 层会有 TOCTOU 窗口。
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
        let _mid = MaterialId::new(material_id)?;
        let _sid = SupplierId::new(supplier_id)?;
        // 计划 §五.4:一个物料只能有一个主供应商。
        // PATCH 不允许把 is_primary 从 false 翻成 true,因为那要求"原来是主的把
        // 主标记取消"+"这条设为主"两步原子操作 —— 这是专门的 set_primary
        // 端点要做的事。PATCH 允许传 is_primary=false(取消主标记)。
        if command.is_primary == Some(true) {
            return Err(AppError::Validation(
                "请使用 POST /materials/{material_id}/suppliers/{supplier_id}/primary 端点设置主供应商,以保证原子地清除其他主标记"
                    .to_string(),
            ));
        }
        // 同样,采购提前期 / 最小采购量 < 0 要拒。
        if let Some(days) = command.lead_time_days {
            if days < 0 {
                return Err(AppError::Validation("采购提前期不能小于 0".to_string()));
            }
        }
        if let Some(moq) = command.moq {
            if moq < 1 {
                return Err(AppError::Validation("最小采购量不能小于 1".to_string()));
            }
        }
        self.material_supplier_repo
            .update_material_supplier(material_id, supplier_id, command)
            .await
    }

    pub async fn set_primary_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<Value> {
        let _mid = MaterialId::new(material_id)?;
        let _sid = SupplierId::new(supplier_id)?;
        // 跨表规则"停用供应商不能设为主供应商"由 postgres.rs::set_primary_supplier
        // 在事务内检查,避免 TOCTOU 与并发竞争。
        self.material_supplier_repo
            .set_primary_supplier(material_id, supplier_id)
            .await
    }

    pub async fn remove_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<Value> {
        let _mid = MaterialId::new(material_id)?;
        let _sid = SupplierId::new(supplier_id)?;
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
        // 计划 §五.6 领域规则:变体编码非空+长度限制、变体名称非空、标准成本 >= 0、
        // 必须绑定有效成品物料、绑定 BOM 必须有效。
        let variant_code = VariantCode::new(command.variant_code.clone())?;
        let base_material_id = MaterialId::new(command.base_material_id.clone())?;
        let _entity = ProductVariant::new(
            variant_code,
            &command.variant_name,
            base_material_id,
            command.standard_cost,
        )?;
        // 跨表规则(base_material 必须 active、bom_id 必须存在且 active)在
        // postgres.rs::create_variant 内事务里检查。
        self.variant_repo.create_variant(command).await
    }

    pub async fn update_variant(
        &self,
        variant_code: &str,
        command: UpdateProductVariantCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        let _id =
            VariantCode::new(variant_code)?;
        self.variant_repo
            .update_variant(variant_code, command)
            .await
    }

    pub async fn activate_variant(&self, variant_code: &str) -> AppResult<Value> {
        let _id =
            VariantCode::new(variant_code)?;
        self.variant_repo.activate_variant(variant_code).await
    }

    pub async fn deactivate_variant(&self, variant_code: &str) -> AppResult<Value> {
        let _id =
            VariantCode::new(variant_code)?;
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
        // 计划 §五.7 领域规则:BOM Header 编码非空+长度限制、bom_name/version 非空。
        let bom_id =
            BomId::new(command.bom_id.clone())?;
        let parent_material_id = MaterialId::new(command.parent_material_id.clone())?;
        let _entity = BomHeader::new(
            bom_id,
            &command.bom_name,
            parent_material_id,
            &command.version,
        )?;
        // valid_to >= valid_from 跨字段规则:命令里两个字段都是 String,
        // 留给 PG 端 CHECK 或 cast 失败兜底;这里宽松处理。
        self.bom_repo.create_bom(command).await
    }

    pub async fn update_bom(
        &self,
        bom_id: &str,
        command: UpdateBomHeaderCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        let _id = BomId::new(bom_id)?;
        self.bom_repo.update_bom(bom_id, command).await
    }

    pub async fn activate_bom(&self, bom_id: &str) -> AppResult<Value> {
        let bom_id = BomId::new(bom_id)?;

        // 1) 加载聚合
        let mut bom = self.bom_repo.load_bom(&bom_id).await?;

        // 2) 聚合内不变式:至少一个组件 → BomNoComponents
        bom.activate()?;

        // 3) 跨聚合不变式:循环依赖
        //    注:即使是 deactivate→activate 的纯状态翻转,这一步也跑一次,
        //    因为这个 BOM 在停用期间,其他 BOM 可能新增了组件,
        //    一旦它再次进入 active 集合,可能就成环了。
        self.bom_repo.assert_no_cycle_after_change(&bom).await?;

        // 4) 持久化
        self.bom_repo.save_bom(&bom).await?;

        Ok(serde_json::json!({
            "success": true,
            "bom_id": bom.id().value(),
            "status": bom.header().status.as_db_value(),
        }))
    }

    pub async fn deactivate_bom(&self, bom_id: &str) -> AppResult<Value> {
        let bom_id = BomId::new(bom_id)?;

        let mut bom = self.bom_repo.load_bom(&bom_id).await?;
        bom.deactivate();
        // 停用不会引入新边,跳过循环检测
        self.bom_repo.save_bom(&bom).await?;

        Ok(serde_json::json!({
            "success": true,
            "bom_id": bom.id().value(),
            "status": bom.header().status.as_db_value(),
        }))
    }

    pub async fn list_components(&self, bom_id: &str) -> AppResult<Value> {
        let _id = BomId::new(bom_id)?;
        self.bom_repo.list_components(bom_id).await
    }

    pub async fn add_component(&self, command: CreateBomComponentCommand) -> AppResult<Value> {
        Self::validate(&command)?;

        // 入参解析(BomComponent::new 把守自引用 / qty>0 / unit 非空)
        let bom_id = BomId::new(command.bom_id.clone())?;
        let parent = MaterialId::new(command.parent_material_id.clone())?;
        let component = MaterialId::new(command.component_material_id.clone())?;
        let new_component = BomComponent::new(
            bom_id.clone(),
            parent,
            component,
            command.quantity,
            &command.unit,
        )?;

        // 1) 加载聚合
        let mut bom = self.bom_repo.load_bom(&bom_id).await?;

        // 2) 聚合内不变式:重复边 → BomComponentDuplicated
        bom.add_component(new_component)?;

        // 3) 跨聚合不变式:循环依赖
        self.bom_repo.assert_no_cycle_after_change(&bom).await?;

        // 4) 持久化(diff-based,事务内)
        self.bom_repo.save_bom(&bom).await?;

        Ok(serde_json::json!({
            "success": true,
            "bom_id": bom.id().value(),
            "component_count": bom.component_count(),
        }))
    }

    pub async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<Value> {
        Self::validate(&command)?;
        // PATCH 只改数量/单位/层级/scrap_rate 等,不改 parent/component 边,
        // 不会引入新的循环引用。但还是要拒绝 quantity <= 0。
        if let Some(q) = command.quantity {
            if q <= rust_decimal::Decimal::ZERO {
                return Err(AppError::Validation("BOM 组件数量必须大于 0".to_string()));
            }
        }
        self.bom_repo.update_component(component_id, command).await
    }

    pub async fn remove_component(&self, component_id: i64) -> AppResult<Value> {
        self.bom_repo.remove_component(component_id).await
    }

    pub async fn get_bom_tree(&self, bom_id: &str) -> AppResult<Value> {
        let _id = BomId::new(bom_id)?;
        self.bom_repo.get_bom_tree(bom_id).await
    }

    pub async fn validate_bom(&self, bom_id: &str) -> AppResult<Value> {
        let _id = BomId::new(bom_id)?;
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
