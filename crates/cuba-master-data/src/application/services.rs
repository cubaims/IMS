use std::sync::Arc;

use validator::Validate;

use super::read_models::*;
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
use crate::domain::{
    BinCode, BomComponent, BomHeader, BomId, BomStatus, Customer, CustomerId, DefectCode,
    DefectCodeMaster, DefectSeverity, InspectionCharId, InspectionCharacteristic, Material,
    MaterialId, MaterialSupplier, MaterialType, ProductVariant, StorageBin, Supplier, SupplierId,
    VariantCode, WorkCenter, WorkCenterId,
};
use cuba_shared::{AppError, AppResult, Page};

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

    fn ensure_not_blank(value: Option<&str>, field: &str) -> AppResult<()> {
        if matches!(value, Some(text) if text.trim().is_empty()) {
            return Err(AppError::Validation(format!("{field} 不能为空")));
        }
        Ok(())
    }

    fn parse_material_type(value: &str) -> AppResult<MaterialType> {
        MaterialType::from_db_value(value).ok_or_else(|| {
            AppError::Validation(format!("未知的物料类型 '{value}',应为 原材料/半成品/成品"))
        })
    }

    fn parse_optional_material_type(value: Option<&str>) -> AppResult<Option<MaterialType>> {
        value.map(Self::parse_material_type).transpose()
    }

    fn parse_defect_severity(value: &str) -> AppResult<DefectSeverity> {
        DefectSeverity::from_db_value(value).ok_or_else(|| {
            AppError::Validation(format!("未知的不良严重等级 '{value}',应为 一般/严重/紧急"))
        })
    }

    fn parse_bom_status(value: &str) -> AppResult<BomStatus> {
        BomStatus::from_db_value(value).ok_or_else(|| {
            AppError::Validation(format!("未知的 BOM 状态 '{value}',应为 草稿/生效/失效"))
        })
    }

    fn ensure_bom_not_activated_directly(value: Option<&str>) -> AppResult<()> {
        if let Some(value) = value {
            let status = Self::parse_bom_status(value)?;
            if matches!(status, BomStatus::Active) {
                return Err(AppError::Validation(
                    "BOM 生效请使用 POST /boms/{bom_id}/activate,以保证组件数量和循环引用校验"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn ensure_non_negative_decimal(
        value: Option<rust_decimal::Decimal>,
        field: &str,
    ) -> AppResult<()> {
        if matches!(value, Some(amount) if amount < rust_decimal::Decimal::ZERO) {
            return Err(AppError::Validation(format!("{field} 不能为负数")));
        }
        Ok(())
    }

    fn build_bom_component(
        command: &CreateBomComponentCommand,
    ) -> AppResult<(BomId, BomComponent)> {
        Self::validate(command)?;

        let bom_id = BomId::new(command.bom_id.clone())?;
        let parent = MaterialId::new(command.parent_material_id.clone())?;
        let component = MaterialId::new(command.component_material_id.clone())?;
        let mut new_component = BomComponent::new(
            bom_id.clone(),
            parent,
            component,
            command.quantity,
            &command.unit,
        )?;

        if matches!(command.bom_level, Some(level) if level < 1) {
            return Err(AppError::Validation("BOM 层级必须大于等于 1".to_string()));
        }
        if let Some(level) = command.bom_level {
            new_component.change_level(level)?;
        }

        Self::ensure_non_negative_decimal(command.scrap_rate, "BOM 组件损耗率")?;
        if let Some(scrap_rate) = command.scrap_rate {
            new_component.change_scrap_rate(scrap_rate)?;
        }

        if let Some(is_critical) = command.is_critical {
            new_component.is_critical = is_critical;
        }

        Ok((bom_id, new_component))
    }

    fn validate_update_bom_component(command: &UpdateBomComponentCommand) -> AppResult<()> {
        Self::validate(command)?;
        if let Some(q) = command.quantity
            && q <= rust_decimal::Decimal::ZERO
        {
            return Err(AppError::Validation("BOM 组件数量必须大于 0".to_string()));
        }
        if matches!(command.bom_level, Some(level) if level < 1) {
            return Err(AppError::Validation("BOM 层级必须大于等于 1".to_string()));
        }
        Self::ensure_non_negative_decimal(command.scrap_rate, "BOM 组件损耗率")
    }

    fn ensure_inspection_type(value: Option<&str>) -> AppResult<()> {
        if let Some(value) = value {
            match value {
                "来料检验" | "过程检验" | "最终检验" => {}
                other => {
                    return Err(AppError::Validation(format!(
                        "未知的检验类型 '{other}',应为 来料检验/过程检验/最终检验"
                    )));
                }
            }
        }
        Ok(())
    }

    fn ensure_positive_decimal(value: Option<rust_decimal::Decimal>, field: &str) -> AppResult<()> {
        if matches!(value, Some(amount) if amount <= rust_decimal::Decimal::ZERO) {
            return Err(AppError::Validation(format!("{field} 必须大于 0")));
        }
        Ok(())
    }

    fn ensure_quality_rating(value: Option<&str>) -> AppResult<()> {
        if let Some(value) = value {
            match value {
                "A" | "B" | "C" | "D" => {}
                other => {
                    return Err(AppError::Validation(format!(
                        "未知的供应商质量等级 '{other}',应为 A/B/C/D"
                    )));
                }
            }
        }
        Ok(())
    }

    pub async fn list_materials(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<MaterialReadModel>> {
        self.material_repo.list_materials(query).await
    }

    pub async fn get_material(&self, material_id: &str) -> AppResult<MaterialReadModel> {
        self.material_repo.get_material(material_id).await
    }

    pub async fn create_material(
        &self,
        command: CreateMaterialCommand,
    ) -> AppResult<MaterialReadModel> {
        Self::validate(&command)?;
        // 计划 §五.1 领域规则:物料编码非空 + 长度限制、名称非空、base_unit 非空、
        // 安全库存/标准成本/MAP >= 0。这些都在 entity 构造里强制。
        let material_id = MaterialId::new(command.material_id.clone())?;
        let material_type = Self::parse_material_type(&command.material_type)?;
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
    ) -> AppResult<MaterialReadModel> {
        Self::validate(&command)?;
        let _id = MaterialId::new(material_id)?;
        let mut entity = Material::new(
            MaterialId::new(material_id)?,
            command.material_name.as_deref().unwrap_or("Material"),
            MaterialType::RawMaterial,
            command.base_unit.as_deref().unwrap_or("EA"),
            command.default_zone.as_deref().unwrap_or("RM"),
            command.safety_stock.unwrap_or(0),
            command.reorder_point.unwrap_or(0),
            command
                .standard_price
                .unwrap_or(rust_decimal::Decimal::ZERO),
            rust_decimal::Decimal::ZERO,
        )?;
        if let Some(name) = command.material_name.as_deref() {
            entity.rename(name)?;
        }
        if let Some(base_unit) = command.base_unit.as_deref() {
            entity.change_base_unit(base_unit)?;
        }
        if let Some(default_zone) = command.default_zone.as_deref() {
            entity.change_default_zone(default_zone)?;
        }
        entity.change_planning_stock(
            command.safety_stock.unwrap_or(entity.safety_stock),
            command.reorder_point.unwrap_or(entity.reorder_point),
        )?;
        if let Some(standard_price) = command.standard_price {
            entity.change_standard_price(standard_price)?;
        }
        if let Some(status) = command.status.as_deref() {
            match status {
                "正常" => entity.activate(),
                "停用" => entity.deactivate(),
                "冻结" => {}
                other => {
                    return Err(AppError::Validation(format!(
                        "未知的物料状态 '{other}',应为 正常/停用/冻结"
                    )));
                }
            }
        }
        self.material_repo
            .update_material(material_id, command)
            .await
    }

    pub async fn activate_material(&self, material_id: &str) -> AppResult<MutationAck> {
        let _id = MaterialId::new(material_id)?;
        self.material_repo.activate_material(material_id).await
    }

    pub async fn deactivate_material(&self, material_id: &str) -> AppResult<MutationAck> {
        let _id = MaterialId::new(material_id)?;
        self.material_repo.deactivate_material(material_id).await
    }

    pub async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Page<StorageBinReadModel>> {
        self.bin_repo.list_bins(query).await
    }

    pub async fn get_bin(&self, bin_code: &str) -> AppResult<StorageBinReadModel> {
        self.bin_repo.get_bin(bin_code).await
    }

    pub async fn create_bin(
        &self,
        command: CreateStorageBinCommand,
    ) -> AppResult<StorageBinReadModel> {
        Self::validate(&command)?;
        // 计划 §五.2 / 执行约定 v1:bin_code 非空+长度限制、zone/bin_type 非空、capacity > 0
        let bin_code = BinCode::new(command.bin_code.clone())?;
        let _entity =
            StorageBin::new(bin_code, &command.zone, &command.bin_type, command.capacity)?;

        self.bin_repo.create_bin(command).await
    }

    pub async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<StorageBinReadModel> {
        Self::validate(&command)?;
        // BinCode::new 在这里只做 ID 格式校验;capacity 跨字段规则
        // (capacity > 0 且 capacity >= current_occupied)在 postgres.rs::update_bin 里
        // 用 StorageBin::change_capacity 跑,因为只在那一层能拿到 current_occupied。
        let _id = BinCode::new(bin_code)?;
        if let Some(zone) = command.zone.as_deref() {
            Self::ensure_not_blank(Some(zone), "货位区域")?;
        }
        if let Some(bin_type) = command.bin_type.as_deref() {
            Self::ensure_not_blank(Some(bin_type), "货位类型")?;
        }
        if let Some(status) = command.status.as_deref() {
            match status {
                "正常" | "占用" | "维护中" | "冻结" => {}
                other => {
                    return Err(AppError::Validation(format!(
                        "未知的货位状态 '{other}',应为 正常/占用/维护中/冻结"
                    )));
                }
            }
        }
        self.bin_repo.update_bin(bin_code, command).await
    }

    pub async fn activate_bin(&self, bin_code: &str) -> AppResult<MutationAck> {
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.activate_bin(bin_code).await
    }

    pub async fn deactivate_bin(&self, bin_code: &str) -> AppResult<MutationAck> {
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.deactivate_bin(bin_code).await
    }

    /// 计划 §五.2:查询货位容量利用率
    pub async fn get_bin_capacity_utilization(
        &self,
        bin_code: &str,
    ) -> AppResult<BinCapacityUtilizationReadModel> {
        let _id = BinCode::new(bin_code)?;
        self.bin_repo.get_bin_capacity_utilization(bin_code).await
    }

    pub async fn list_suppliers(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<SupplierReadModel>> {
        self.supplier_repo.list_suppliers(query).await
    }

    pub async fn get_supplier(&self, supplier_id: &str) -> AppResult<SupplierReadModel> {
        self.supplier_repo.get_supplier(supplier_id).await
    }

    pub async fn create_supplier(
        &self,
        command: CreateSupplierCommand,
    ) -> AppResult<SupplierReadModel> {
        Self::validate(&command)?;
        // 计划 §五.3 领域规则:供应商编码非空+长度限制、供应商名称非空
        let supplier_id = SupplierId::new(command.supplier_id.clone())?;
        let _entity = Supplier::new(supplier_id, &command.supplier_name)?;
        Self::ensure_quality_rating(command.quality_rating.as_deref())?;

        self.supplier_repo.create_supplier(command).await
    }

    pub async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<SupplierReadModel> {
        Self::validate(&command)?;
        let _id = SupplierId::new(supplier_id)?;
        if let Some(name) = command.supplier_name.as_deref() {
            let mut entity = Supplier::new(SupplierId::new(supplier_id)?, "Supplier")?;
            entity.rename(name)?;
        }
        Self::ensure_quality_rating(command.quality_rating.as_deref())?;
        self.supplier_repo
            .update_supplier(supplier_id, command)
            .await
    }

    pub async fn activate_supplier(&self, supplier_id: &str) -> AppResult<MutationAck> {
        let _id = SupplierId::new(supplier_id)?;
        self.supplier_repo.activate_supplier(supplier_id).await
    }

    pub async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<MutationAck> {
        let _id = SupplierId::new(supplier_id)?;
        self.supplier_repo.deactivate_supplier(supplier_id).await
    }

    pub async fn list_customers(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<CustomerReadModel>> {
        self.customer_repo.list_customers(query).await
    }

    pub async fn get_customer(&self, customer_id: &str) -> AppResult<CustomerReadModel> {
        self.customer_repo.get_customer(customer_id).await
    }

    pub async fn create_customer(
        &self,
        command: CreateCustomerCommand,
    ) -> AppResult<CustomerReadModel> {
        Self::validate(&command)?;
        // 计划 §五.5 领域规则:客户编码非空+长度限制、客户名称非空、信用额度 >= 0。
        let customer_id = CustomerId::new(command.customer_id.clone())?;
        let mut entity = Customer::new(customer_id, &command.customer_name)?;
        if let Some(credit_limit) = command.credit_limit {
            entity.change_credit_limit(credit_limit)?;
        }

        self.customer_repo.create_customer(command).await
    }

    pub async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<CustomerReadModel> {
        Self::validate(&command)?;
        let _id = CustomerId::new(customer_id)?;
        if let Some(name) = command.customer_name.as_deref() {
            let mut entity = Customer::new(CustomerId::new(customer_id)?, "Customer")?;
            entity.rename(name)?;
        }
        if let Some(credit_limit) = command.credit_limit {
            let mut entity = Customer::new(CustomerId::new(customer_id)?, "Customer")?;
            entity.change_credit_limit(credit_limit)?;
        }
        self.customer_repo
            .update_customer(customer_id, command)
            .await
    }

    pub async fn activate_customer(&self, customer_id: &str) -> AppResult<MutationAck> {
        let _id = CustomerId::new(customer_id)?;
        self.customer_repo.activate_customer(customer_id).await
    }

    pub async fn deactivate_customer(&self, customer_id: &str) -> AppResult<MutationAck> {
        let _id = CustomerId::new(customer_id)?;
        self.customer_repo.deactivate_customer(customer_id).await
    }

    pub async fn list_material_suppliers(
        &self,
        material_id: &str,
    ) -> AppResult<Vec<MaterialSupplierReadModel>> {
        let _id = MaterialId::new(material_id)?;
        self.material_supplier_repo
            .list_material_suppliers(material_id)
            .await
    }

    pub async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<MaterialSupplierReadModel> {
        Self::validate(&command)?;
        // 计划 §五.4 / 执行约定 v1 领域规则:采购提前期 >= 0、最小采购量 >= 1。
        let material_id = MaterialId::new(command.material_id.clone())?;
        let supplier_id = SupplierId::new(command.supplier_id.clone())?;
        let _entity = MaterialSupplier::new(
            material_id,
            supplier_id,
            command.is_primary.unwrap_or(false),
            command.lead_time_days.unwrap_or(0),
            command.moq.unwrap_or(1),
        )?;
        Self::ensure_non_negative_decimal(command.purchase_price, "采购价格")?;
        Self::ensure_quality_rating(command.quality_rating.as_deref())?;
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
    ) -> AppResult<MaterialSupplierReadModel> {
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
        Self::ensure_non_negative_decimal(command.purchase_price, "采购价格")?;
        Self::ensure_quality_rating(command.quality_rating.as_deref())?;
        self.material_supplier_repo
            .update_material_supplier(material_id, supplier_id, command)
            .await
    }

    pub async fn set_primary_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<MaterialSupplierReadModel> {
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
    ) -> AppResult<DeleteAck> {
        let _mid = MaterialId::new(material_id)?;
        let _sid = SupplierId::new(supplier_id)?;
        self.material_supplier_repo
            .remove_material_supplier(material_id, supplier_id)
            .await
    }

    pub async fn list_variants(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<ProductVariantReadModel>> {
        self.variant_repo.list_variants(query).await
    }

    pub async fn get_variant(&self, variant_code: &str) -> AppResult<ProductVariantReadModel> {
        self.variant_repo.get_variant(variant_code).await
    }

    pub async fn create_variant(
        &self,
        command: CreateProductVariantCommand,
    ) -> AppResult<ProductVariantReadModel> {
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
    ) -> AppResult<ProductVariantReadModel> {
        Self::validate(&command)?;
        let _id = VariantCode::new(variant_code)?;
        let mut entity = ProductVariant::new(
            VariantCode::new(variant_code)?,
            command.variant_name.as_deref().unwrap_or("Variant"),
            MaterialId::new("M")?,
            command.standard_cost.unwrap_or(rust_decimal::Decimal::ZERO),
        )?;
        if let Some(name) = command.variant_name.as_deref() {
            entity.rename(name)?;
        }
        if let Some(bom_id) = command.bom_id.as_deref() {
            entity.bind_bom(BomId::new(bom_id)?);
        }
        if let Some(standard_cost) = command.standard_cost {
            entity.change_standard_cost(standard_cost)?;
        }
        if let Some(is_active) = command.is_active {
            if is_active {
                entity.activate();
            } else {
                entity.deactivate();
            }
        }
        self.variant_repo
            .update_variant(variant_code, command)
            .await
    }

    pub async fn activate_variant(&self, variant_code: &str) -> AppResult<MutationAck> {
        let _id = VariantCode::new(variant_code)?;
        self.variant_repo.activate_variant(variant_code).await
    }

    pub async fn deactivate_variant(&self, variant_code: &str) -> AppResult<MutationAck> {
        let _id = VariantCode::new(variant_code)?;
        self.variant_repo.deactivate_variant(variant_code).await
    }

    pub async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Page<BomSummaryReadModel>> {
        self.bom_repo.list_boms(query).await
    }

    pub async fn get_bom(&self, bom_id: &str) -> AppResult<BomDetailReadModel> {
        self.bom_repo.get_bom(bom_id).await
    }

    pub async fn create_bom(
        &self,
        command: CreateBomHeaderCommand,
    ) -> AppResult<BomHeaderReadModel> {
        Self::validate(&command)?;
        // 计划 §五.7 领域规则:BOM Header 编码非空+长度限制、bom_name/version 非空。
        let bom_id = BomId::new(command.bom_id.clone())?;
        let parent_material_id = MaterialId::new(command.parent_material_id.clone())?;
        let _entity = BomHeader::new(
            bom_id,
            &command.bom_name,
            parent_material_id,
            &command.version,
        )?;
        Self::ensure_positive_decimal(command.base_quantity, "BOM 基准数量")?;
        Self::ensure_bom_not_activated_directly(command.status.as_deref())?;
        // valid_to >= valid_from 跨字段规则:命令里两个字段都是 String,
        // 留给 PG 端 CHECK 或 cast 失败兜底;这里宽松处理。
        self.bom_repo.create_bom(command).await
    }

    pub async fn update_bom(
        &self,
        bom_id: &str,
        command: UpdateBomHeaderCommand,
    ) -> AppResult<BomHeaderReadModel> {
        Self::validate(&command)?;
        let _id = BomId::new(bom_id)?;
        if let Some(name) = command.bom_name.as_deref() {
            Self::ensure_not_blank(Some(name), "BOM 名称")?;
        }
        if let Some(version) = command.version.as_deref() {
            Self::ensure_not_blank(Some(version), "BOM 版本")?;
        }
        Self::ensure_positive_decimal(command.base_quantity, "BOM 基准数量")?;
        Self::ensure_bom_not_activated_directly(command.status.as_deref())?;
        self.bom_repo.update_bom(bom_id, command).await
    }

    pub async fn activate_bom(&self, bom_id: &str) -> AppResult<BomLifecycleReadModel> {
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

        Ok(BomLifecycleReadModel {
            success: true,
            bom_id: bom.id().value().to_string(),
            status: bom.header().status.as_db_value().to_string(),
        })
    }

    pub async fn deactivate_bom(&self, bom_id: &str) -> AppResult<BomLifecycleReadModel> {
        let bom_id = BomId::new(bom_id)?;

        let mut bom = self.bom_repo.load_bom(&bom_id).await?;
        bom.deactivate();
        // 停用不会引入新边,跳过循环检测
        self.bom_repo.save_bom(&bom).await?;

        Ok(BomLifecycleReadModel {
            success: true,
            bom_id: bom.id().value().to_string(),
            status: bom.header().status.as_db_value().to_string(),
        })
    }

    pub async fn list_components(&self, bom_id: &str) -> AppResult<Vec<BomComponentReadModel>> {
        let _id = BomId::new(bom_id)?;
        self.bom_repo.list_components(bom_id).await
    }

    pub async fn add_component(
        &self,
        command: CreateBomComponentCommand,
    ) -> AppResult<BomComponentCountReadModel> {
        // 入参解析(BomComponent::new 把守自引用 / qty>0 / unit 非空),
        // 可选属性必须落入聚合后再持久化,避免 API 接收但保存默认值。
        let (bom_id, new_component) = Self::build_bom_component(&command)?;

        // 1) 加载聚合
        let mut bom = self.bom_repo.load_bom(&bom_id).await?;

        // 2) 聚合内不变式:重复边 → BomComponentDuplicated
        bom.add_component(new_component)?;

        // 3) 跨聚合不变式:循环依赖
        self.bom_repo.assert_no_cycle_after_change(&bom).await?;

        // 4) 持久化(diff-based,事务内)
        self.bom_repo.save_bom(&bom).await?;

        Ok(BomComponentCountReadModel {
            success: true,
            bom_id: bom.id().value().to_string(),
            component_count: bom.component_count(),
        })
    }

    pub async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<BomComponentReadModel> {
        // PATCH 只改数量/单位/层级/scrap_rate 等,不改 parent/component 边,
        // 不会引入新的循环引用。但还是要拒绝 quantity <= 0。
        Self::validate_update_bom_component(&command)?;
        self.bom_repo.update_component(component_id, command).await
    }

    pub async fn update_component_for_bom(
        &self,
        bom_id: &str,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<BomComponentReadModel> {
        let _id = BomId::new(bom_id)?;
        Self::validate_update_bom_component(&command)?;
        self.bom_repo
            .update_component_for_bom(bom_id, component_id, command)
            .await
    }

    pub async fn remove_component(&self, component_id: i64) -> AppResult<DeleteAck> {
        self.bom_repo.remove_component(component_id).await
    }

    pub async fn remove_component_for_bom(
        &self,
        bom_id: &str,
        component_id: i64,
    ) -> AppResult<DeleteAck> {
        let _id = BomId::new(bom_id)?;
        self.bom_repo
            .remove_component_for_bom(bom_id, component_id)
            .await
    }

    pub async fn get_bom_tree(&self, bom_id: &str) -> AppResult<BomTreeReadModel> {
        let _id = BomId::new(bom_id)?;
        self.bom_repo.get_bom_tree(bom_id).await
    }

    pub async fn validate_bom(&self, bom_id: &str) -> AppResult<BomValidationReadModel> {
        let _id = BomId::new(bom_id)?;
        self.bom_repo.validate_bom(bom_id).await
    }

    pub async fn preview_bom_explosion(
        &self,
        material_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<BomExplosionPreviewReadModel> {
        let _id = MaterialId::new(material_id)?;
        if quantity <= 0 {
            return Err(AppError::Validation("BOM 试算数量必须大于 0".to_string()));
        }
        if let Some(variant_code) = variant_code.as_deref() {
            let _variant = VariantCode::new(variant_code)?;
        }
        self.bom_repo
            .preview_bom_explosion(material_id, quantity, variant_code)
            .await
    }

    pub async fn preview_bom_explosion_for_bom(
        &self,
        bom_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<BomExplosionPreviewReadModel> {
        let bom_id = BomId::new(bom_id)?;
        if quantity <= 0 {
            return Err(AppError::Validation("BOM 试算数量必须大于 0".to_string()));
        }
        let bom = self.bom_repo.load_bom(&bom_id).await?;
        let variant_code = variant_code.or_else(|| {
            bom.header()
                .variant_code
                .as_ref()
                .map(|code| code.value().to_string())
        });
        self.bom_repo
            .preview_bom_explosion(
                bom.header().parent_material_id.value(),
                quantity,
                variant_code,
            )
            .await
    }

    pub async fn list_work_centers(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<WorkCenterReadModel>> {
        self.work_center_repo.list_work_centers(query).await
    }

    pub async fn get_work_center(&self, work_center_id: &str) -> AppResult<WorkCenterReadModel> {
        self.work_center_repo.get_work_center(work_center_id).await
    }

    pub async fn create_work_center(
        &self,
        command: CreateWorkCenterCommand,
    ) -> AppResult<WorkCenterReadModel> {
        Self::validate(&command)?;
        let work_center_id = WorkCenterId::new(command.work_center_id.clone())?;
        let mut entity = WorkCenter::new(work_center_id, &command.work_center_name)?;
        entity.change_capacity(command.capacity_per_day)?;
        if let Some(efficiency) = command.efficiency {
            entity.change_efficiency(efficiency)?;
        }
        self.work_center_repo.create_work_center(command).await
    }

    pub async fn update_work_center(
        &self,
        work_center_id: &str,
        command: UpdateWorkCenterCommand,
    ) -> AppResult<WorkCenterReadModel> {
        Self::validate(&command)?;
        let _id = WorkCenterId::new(work_center_id)?;
        if let Some(name) = command.work_center_name.as_deref() {
            let mut entity = WorkCenter::new(WorkCenterId::new(work_center_id)?, "Work Center")?;
            entity.rename(name)?;
        }
        let mut entity = WorkCenter::new(WorkCenterId::new(work_center_id)?, "Work Center")?;
        entity.change_capacity(command.capacity_per_day)?;
        if let Some(efficiency) = command.efficiency {
            entity.change_efficiency(efficiency)?;
        }
        self.work_center_repo
            .update_work_center(work_center_id, command)
            .await
    }

    pub async fn activate_work_center(&self, work_center_id: &str) -> AppResult<MutationAck> {
        let _id = WorkCenterId::new(work_center_id)?;
        self.work_center_repo
            .activate_work_center(work_center_id)
            .await
    }

    pub async fn deactivate_work_center(&self, work_center_id: &str) -> AppResult<MutationAck> {
        let _id = WorkCenterId::new(work_center_id)?;
        self.work_center_repo
            .deactivate_work_center(work_center_id)
            .await
    }

    pub async fn list_inspection_chars(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<InspectionCharacteristicReadModel>> {
        self.quality_master_repo.list_inspection_chars(query).await
    }

    pub async fn get_inspection_char(
        &self,
        char_id: &str,
    ) -> AppResult<InspectionCharacteristicReadModel> {
        self.quality_master_repo.get_inspection_char(char_id).await
    }

    pub async fn create_inspection_char(
        &self,
        command: CreateInspectionCharCommand,
    ) -> AppResult<InspectionCharacteristicReadModel> {
        Self::validate(&command)?;
        let char_id = InspectionCharId::new(command.char_id.clone())?;
        let material_type = Self::parse_optional_material_type(command.material_type.as_deref())?;
        Self::ensure_inspection_type(command.inspection_type.as_deref())?;
        let mut entity = InspectionCharacteristic::new(char_id, &command.char_name)?;
        entity.material_type = material_type;
        entity.set_limits(command.lower_limit, command.upper_limit)?;
        self.quality_master_repo
            .create_inspection_char(command)
            .await
    }

    pub async fn update_inspection_char(
        &self,
        char_id: &str,
        command: UpdateInspectionCharCommand,
    ) -> AppResult<InspectionCharacteristicReadModel> {
        Self::validate(&command)?;
        let _id = InspectionCharId::new(char_id)?;
        Self::parse_optional_material_type(command.material_type.as_deref())?;
        Self::ensure_inspection_type(command.inspection_type.as_deref())?;
        if let Some(name) = command.char_name.as_deref() {
            let mut entity =
                InspectionCharacteristic::new(InspectionCharId::new(char_id)?, "Inspection")?;
            entity.rename(name)?;
        }
        if command.lower_limit.is_some() && command.upper_limit.is_some() {
            let mut entity =
                InspectionCharacteristic::new(InspectionCharId::new(char_id)?, "Inspection")?;
            entity.set_limits(command.lower_limit, command.upper_limit)?;
        }
        self.quality_master_repo
            .update_inspection_char(char_id, command)
            .await
    }

    pub async fn activate_inspection_char(&self, char_id: &str) -> AppResult<MutationAck> {
        let _id = InspectionCharId::new(char_id)?;
        self.quality_master_repo
            .activate_inspection_char(char_id)
            .await
    }

    pub async fn deactivate_inspection_char(&self, char_id: &str) -> AppResult<MutationAck> {
        let _id = InspectionCharId::new(char_id)?;
        self.quality_master_repo
            .deactivate_inspection_char(char_id)
            .await
    }

    pub async fn list_defect_codes(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<DefectCodeReadModel>> {
        self.quality_master_repo.list_defect_codes(query).await
    }

    pub async fn get_defect_code(&self, defect_code: &str) -> AppResult<DefectCodeReadModel> {
        self.quality_master_repo.get_defect_code(defect_code).await
    }

    pub async fn create_defect_code(
        &self,
        command: CreateDefectCodeCommand,
    ) -> AppResult<DefectCodeReadModel> {
        Self::validate(&command)?;
        let defect_code = DefectCode::new(command.defect_code.clone())?;
        let severity = Self::parse_defect_severity(&command.severity)?;
        let _entity = DefectCodeMaster::new(defect_code, &command.defect_name, severity)?;
        self.quality_master_repo.create_defect_code(command).await
    }

    pub async fn update_defect_code(
        &self,
        defect_code: &str,
        command: UpdateDefectCodeCommand,
    ) -> AppResult<DefectCodeReadModel> {
        Self::validate(&command)?;
        let _id = DefectCode::new(defect_code)?;
        if let Some(name) = command.defect_name.as_deref() {
            let mut entity = DefectCodeMaster::new(
                DefectCode::new(defect_code)?,
                "Defect",
                DefectSeverity::Minor,
            )?;
            entity.rename(name)?;
        }
        if let Some(severity) = command.severity.as_deref() {
            let mut entity = DefectCodeMaster::new(
                DefectCode::new(defect_code)?,
                "Defect",
                DefectSeverity::Minor,
            )?;
            entity.change_severity(Self::parse_defect_severity(severity)?);
        }
        self.quality_master_repo
            .update_defect_code(defect_code, command)
            .await
    }

    pub async fn activate_defect_code(&self, defect_code: &str) -> AppResult<MutationAck> {
        let _id = DefectCode::new(defect_code)?;
        self.quality_master_repo
            .activate_defect_code(defect_code)
            .await
    }

    pub async fn deactivate_defect_code(&self, defect_code: &str) -> AppResult<MutationAck> {
        let _id = DefectCode::new(defect_code)?;
        self.quality_master_repo
            .deactivate_defect_code(defect_code)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::*;

    fn d(value: &str) -> Decimal {
        Decimal::from_str(value).expect("test fixture should be valid")
    }

    fn bom_component_command() -> CreateBomComponentCommand {
        CreateBomComponentCommand {
            bom_id: "BOM001".to_string(),
            parent_material_id: "PARENT001".to_string(),
            component_material_id: "COMP001".to_string(),
            quantity: d("2.5"),
            unit: "EA".to_string(),
            bom_level: Some(3),
            scrap_rate: Some(d("0.125")),
            is_critical: Some(true),
        }
    }

    fn create_bin_command(capacity: i32) -> CreateStorageBinCommand {
        CreateStorageBinCommand {
            bin_code: "BIN001".to_string(),
            zone: "RM".to_string(),
            bin_type: "普通货位".to_string(),
            capacity,
            notes: None,
        }
    }

    fn update_inspection_command(
        lower_limit: Option<Decimal>,
        upper_limit: Option<Decimal>,
    ) -> UpdateInspectionCharCommand {
        UpdateInspectionCharCommand {
            char_name: None,
            material_type: None,
            inspection_type: None,
            method: None,
            standard: None,
            unit: None,
            lower_limit,
            upper_limit,
            is_critical: None,
        }
    }

    #[test]
    fn build_bom_component_applies_optional_fields() {
        let (_, component) = MasterDataService::build_bom_component(&bom_component_command())
            .expect("test fixture should be valid");

        assert_eq!(component.quantity, d("2.5"));
        assert_eq!(component.unit, "EA");
        assert_eq!(component.bom_level, 3);
        assert_eq!(component.scrap_rate, d("0.125"));
        assert!(component.is_critical);
    }

    #[test]
    fn build_bom_component_rejects_invalid_level() {
        let mut command = bom_component_command();
        command.bom_level = Some(0);

        let err = MasterDataService::build_bom_component(&command)
            .expect_err("invalid BOM level should fail validation");

        assert!(matches!(
            err,
            AppError::Validation(message) if message.contains("BOM 层级")
        ));
    }

    #[test]
    fn create_bin_allows_zero_to_reach_domain_capacity_code() {
        let command = create_bin_command(0);

        MasterDataService::validate(&command).expect("zero capacity is a domain business error");
        let err: AppError = StorageBin::new(
            BinCode::new(command.bin_code).expect("test fixture should be valid"),
            command.zone,
            command.bin_type,
            command.capacity,
        )
        .expect_err("zero capacity should fail")
        .into();

        assert!(matches!(
            err,
            AppError::Business { code, .. } if code == "BIN_CAPACITY_INVALID"
        ));
    }

    #[test]
    fn create_bin_rejects_negative_capacity_at_dto_layer() {
        let command = create_bin_command(-1);

        let err = MasterDataService::validate(&command)
            .expect_err("negative capacity should fail validation");

        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn update_inspection_limits_reject_inverted_pair() {
        let command = update_inspection_command(Some(d("10.2")), Some(d("9.8")));
        let mut entity = InspectionCharacteristic::new(
            InspectionCharId::new("IC001").expect("test fixture should be valid"),
            "Dimension",
        )
        .expect("test fixture should be valid");

        let err: AppError = entity
            .set_limits(command.lower_limit, command.upper_limit)
            .expect_err("inverted inspection limits should fail")
            .into();

        assert!(matches!(
            err,
            AppError::Business { code, .. } if code == "INSPECTION_LIMIT_INVALID"
        ));
    }

    #[test]
    fn update_inspection_limits_accept_one_sided_command() {
        let command = update_inspection_command(None, Some(d("9.8")));

        MasterDataService::validate(&command)
            .expect("one-sided limit command is syntactically valid");
    }
}
