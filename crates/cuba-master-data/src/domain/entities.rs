use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{
    BinCode, BomId, BomStatus, CustomerId, DefectCode, DefectSeverity, InspectionCharId,
    MasterDataDomainError, MaterialId, MaterialType, SupplierId, VariantCode, WorkCenterId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub material_id: MaterialId,
    pub material_name: String,
    pub material_type: MaterialType,
    pub base_unit: String,
    pub default_zone: String,
    pub safety_stock: i32,
    pub reorder_point: i32,
    pub standard_price: Decimal,
    pub map_price: Decimal,
    pub current_stock: i32,
    pub status: String,
}

impl Material {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        material_id: MaterialId,
        material_name: impl Into<String>,
        material_type: MaterialType,
        base_unit: impl Into<String>,
        default_zone: impl Into<String>,
        safety_stock: i32,
        reorder_point: i32,
        standard_price: Decimal,
        map_price: Decimal,
    ) -> Result<Self, MasterDataDomainError> {
        let material_name = material_name.into().trim().to_string();
        let base_unit = base_unit.into().trim().to_string();
        let default_zone = default_zone.into().trim().to_string();

        if material_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        if base_unit.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        if safety_stock < 0 || reorder_point < 0 {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }

        if standard_price < Decimal::ZERO || map_price < Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }

        Ok(Self {
            material_id,
            material_name,
            material_type,
            base_unit,
            default_zone,
            safety_stock,
            reorder_point,
            standard_price,
            map_price,
            current_stock: 0,
            status: "正常".to_string(),
        })
    }

    pub fn rename(
        &mut self,
        material_name: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let material_name = material_name.into().trim().to_string();
        if material_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.material_name = material_name;
        Ok(())
    }

    pub fn change_base_unit(
        &mut self,
        base_unit: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let base_unit = base_unit.into().trim().to_string();
        if base_unit.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.base_unit = base_unit;
        Ok(())
    }

    pub fn change_default_zone(
        &mut self,
        default_zone: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let default_zone = default_zone.into().trim().to_string();
        if default_zone.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.default_zone = default_zone;
        Ok(())
    }

    pub fn change_planning_stock(
        &mut self,
        safety_stock: i32,
        reorder_point: i32,
    ) -> Result<(), MasterDataDomainError> {
        if safety_stock < 0 || reorder_point < 0 {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.safety_stock = safety_stock;
        self.reorder_point = reorder_point;
        Ok(())
    }

    pub fn change_standard_price(
        &mut self,
        standard_price: Decimal,
    ) -> Result<(), MasterDataDomainError> {
        if standard_price < Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.standard_price = standard_price;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.status = "正常".to_string();
    }

    pub fn deactivate(&mut self) {
        self.status = "冻结".to_string();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageBin {
    pub bin_code: BinCode,
    pub zone: String,
    pub bin_type: String,
    pub capacity: i32,
    pub current_occupied: i32,
    pub status: String,
}

impl StorageBin {
    pub fn new(
        bin_code: BinCode,
        zone: impl Into<String>,
        bin_type: impl Into<String>,
        capacity: i32,
    ) -> Result<Self, MasterDataDomainError> {
        let zone = zone.into().trim().to_string();
        let bin_type = bin_type.into().trim().to_string();

        if zone.is_empty() || bin_type.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        if capacity < 0 {
            return Err(MasterDataDomainError::CapacityCannotBeNegative);
        }

        if capacity == 0 {
            return Err(MasterDataDomainError::BinCapacityInvalid);
        }

        Ok(Self {
            bin_code,
            zone,
            bin_type,
            capacity,
            current_occupied: 0,
            status: "正常".to_string(),
        })
    }

    pub fn change_capacity(&mut self, capacity: i32) -> Result<(), MasterDataDomainError> {
        if capacity < 0 {
            return Err(MasterDataDomainError::CapacityCannotBeNegative);
        }

        if capacity == 0 {
            return Err(MasterDataDomainError::BinCapacityInvalid);
        }

        if capacity < self.current_occupied {
            return Err(MasterDataDomainError::CapacityCannotBeLessThanOccupied);
        }

        self.capacity = capacity;
        Ok(())
    }

    pub fn change_zone(&mut self, zone: impl Into<String>) -> Result<(), MasterDataDomainError> {
        let zone = zone.into().trim().to_string();
        if zone.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.zone = zone;
        Ok(())
    }

    pub fn change_type(
        &mut self,
        bin_type: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let bin_type = bin_type.into().trim().to_string();
        if bin_type.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.bin_type = bin_type;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.status = "正常".to_string();
    }

    pub fn deactivate(&mut self) {
        self.status = "冻结".to_string();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    pub supplier_id: SupplierId,
    pub supplier_name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub quality_rating: String,
    pub is_active: bool,
}

impl Supplier {
    pub fn new(
        supplier_id: SupplierId,
        supplier_name: impl Into<String>,
    ) -> Result<Self, MasterDataDomainError> {
        let supplier_name = supplier_name.into().trim().to_string();

        if supplier_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            supplier_id,
            supplier_name,
            contact_person: None,
            phone: None,
            email: None,
            address: None,
            quality_rating: "A".to_string(),
            is_active: true,
        })
    }

    pub fn rename(
        &mut self,
        supplier_name: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let supplier_name = supplier_name.into().trim().to_string();
        if supplier_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.supplier_name = supplier_name;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub customer_id: CustomerId,
    pub customer_name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub credit_limit: Decimal,
    pub is_active: bool,
}

impl Customer {
    pub fn new(
        customer_id: CustomerId,
        customer_name: impl Into<String>,
    ) -> Result<Self, MasterDataDomainError> {
        let customer_name = customer_name.into().trim().to_string();

        if customer_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            customer_id,
            customer_name,
            contact_person: None,
            phone: None,
            email: None,
            address: None,
            credit_limit: Decimal::ZERO,
            is_active: true,
        })
    }

    pub fn rename(
        &mut self,
        customer_name: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let customer_name = customer_name.into().trim().to_string();
        if customer_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.customer_name = customer_name;
        Ok(())
    }

    pub fn change_credit_limit(
        &mut self,
        credit_limit: Decimal,
    ) -> Result<(), MasterDataDomainError> {
        if credit_limit < Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.credit_limit = credit_limit;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSupplier {
    pub material_id: MaterialId,
    pub supplier_id: SupplierId,
    pub is_primary: bool,
    pub purchase_price: Option<Decimal>,
    pub lead_time_days: i32,
    pub moq: i32,
    pub is_active: bool,
}

impl MaterialSupplier {
    pub fn new(
        material_id: MaterialId,
        supplier_id: SupplierId,
        is_primary: bool,
        lead_time_days: i32,
        moq: i32,
    ) -> Result<Self, MasterDataDomainError> {
        if lead_time_days < 0 {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }

        if moq < 1 {
            return Err(MasterDataDomainError::QuantityMustBeGreaterThanZero);
        }

        Ok(Self {
            material_id,
            supplier_id,
            is_primary,
            purchase_price: None,
            lead_time_days,
            moq,
            is_active: true,
        })
    }

    pub fn change_lead_time(&mut self, lead_time_days: i32) -> Result<(), MasterDataDomainError> {
        if lead_time_days < 0 {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.lead_time_days = lead_time_days;
        Ok(())
    }

    pub fn change_moq(&mut self, moq: i32) -> Result<(), MasterDataDomainError> {
        if moq < 1 {
            return Err(MasterDataDomainError::QuantityMustBeGreaterThanZero);
        }
        self.moq = moq;
        Ok(())
    }

    pub fn mark_primary(&mut self) {
        self.is_primary = true;
    }

    pub fn clear_primary(&mut self) {
        self.is_primary = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub variant_code: VariantCode,
    pub variant_name: String,
    pub base_material_id: MaterialId,
    pub bom_id: Option<BomId>,
    pub standard_cost: Decimal,
    pub is_active: bool,
}

impl ProductVariant {
    pub fn new(
        variant_code: VariantCode,
        variant_name: impl Into<String>,
        base_material_id: MaterialId,
        standard_cost: Decimal,
    ) -> Result<Self, MasterDataDomainError> {
        let variant_name = variant_name.into().trim().to_string();

        if variant_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        if standard_cost < Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }

        Ok(Self {
            variant_code,
            variant_name,
            base_material_id,
            bom_id: None,
            standard_cost,
            is_active: true,
        })
    }

    pub fn rename(&mut self, variant_name: impl Into<String>) -> Result<(), MasterDataDomainError> {
        let variant_name = variant_name.into().trim().to_string();
        if variant_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.variant_name = variant_name;
        Ok(())
    }

    pub fn bind_bom(&mut self, bom_id: BomId) {
        self.bom_id = Some(bom_id);
    }

    pub fn change_standard_cost(
        &mut self,
        standard_cost: Decimal,
    ) -> Result<(), MasterDataDomainError> {
        if standard_cost < Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.standard_cost = standard_cost;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomHeader {
    pub bom_id: BomId,
    pub bom_name: String,
    pub parent_material_id: MaterialId,
    pub variant_code: Option<VariantCode>,
    pub version: String,
    pub status: BomStatus,
    pub is_active: bool,
}

impl BomHeader {
    pub fn new(
        bom_id: BomId,
        bom_name: impl Into<String>,
        parent_material_id: MaterialId,
        version: impl Into<String>,
    ) -> Result<Self, MasterDataDomainError> {
        let bom_name = bom_name.into().trim().to_string();
        let version = version.into().trim().to_string();

        if bom_name.is_empty() || version.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            bom_id,
            bom_name,
            parent_material_id,
            variant_code: None,
            version,
            status: BomStatus::Draft,
            is_active: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomComponent {
    pub bom_id: BomId,
    pub parent_material_id: MaterialId,
    pub component_material_id: MaterialId,
    pub quantity: Decimal,
    pub unit: String,
    pub bom_level: i32,
    pub scrap_rate: Decimal,
    pub is_critical: bool,
}

impl BomComponent {
    pub fn new(
        bom_id: BomId,
        parent_material_id: MaterialId,
        component_material_id: MaterialId,
        quantity: Decimal,
        unit: impl Into<String>,
    ) -> Result<Self, MasterDataDomainError> {
        if parent_material_id == component_material_id {
            return Err(MasterDataDomainError::BomComponentCannotReferenceItself);
        }

        if quantity <= Decimal::ZERO {
            return Err(MasterDataDomainError::QuantityMustBeGreaterThanZero);
        }

        let unit = unit.into().trim().to_string();

        if unit.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            bom_id,
            parent_material_id,
            component_material_id,
            quantity,
            unit,
            bom_level: 1,
            scrap_rate: Decimal::ZERO,
            is_critical: false,
        })
    }

    pub fn change_quantity(&mut self, quantity: Decimal) -> Result<(), MasterDataDomainError> {
        if quantity <= Decimal::ZERO {
            return Err(MasterDataDomainError::QuantityMustBeGreaterThanZero);
        }
        self.quantity = quantity;
        Ok(())
    }

    pub fn change_unit(&mut self, unit: impl Into<String>) -> Result<(), MasterDataDomainError> {
        let unit = unit.into().trim().to_string();
        if unit.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.unit = unit;
        Ok(())
    }

    pub fn change_level(&mut self, bom_level: i32) -> Result<(), MasterDataDomainError> {
        if bom_level < 1 {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.bom_level = bom_level;
        Ok(())
    }

    pub fn change_scrap_rate(&mut self, scrap_rate: Decimal) -> Result<(), MasterDataDomainError> {
        if scrap_rate < Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.scrap_rate = scrap_rate;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCenter {
    pub work_center_id: WorkCenterId,
    pub work_center_name: String,
    pub location: Option<String>,
    pub capacity_per_day: Option<i32>,
    pub efficiency: Decimal,
    pub is_active: bool,
}

impl WorkCenter {
    pub fn new(
        work_center_id: WorkCenterId,
        work_center_name: impl Into<String>,
    ) -> Result<Self, MasterDataDomainError> {
        let work_center_name = work_center_name.into().trim().to_string();

        if work_center_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            work_center_id,
            work_center_name,
            location: None,
            capacity_per_day: None,
            efficiency: Decimal::new(10000, 2),
            is_active: true,
        })
    }

    pub fn rename(
        &mut self,
        work_center_name: impl Into<String>,
    ) -> Result<(), MasterDataDomainError> {
        let work_center_name = work_center_name.into().trim().to_string();
        if work_center_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.work_center_name = work_center_name;
        Ok(())
    }

    pub fn change_capacity(
        &mut self,
        capacity_per_day: Option<i32>,
    ) -> Result<(), MasterDataDomainError> {
        if matches!(capacity_per_day, Some(capacity) if capacity <= 0) {
            return Err(MasterDataDomainError::QuantityMustBeGreaterThanZero);
        }
        self.capacity_per_day = capacity_per_day;
        Ok(())
    }

    pub fn change_efficiency(&mut self, efficiency: Decimal) -> Result<(), MasterDataDomainError> {
        if efficiency <= Decimal::ZERO {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
        }
        self.efficiency = efficiency;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionCharacteristic {
    pub char_id: InspectionCharId,
    pub char_name: String,
    pub material_type: Option<MaterialType>,
    pub inspection_type: Option<String>,
    pub method: Option<String>,
    pub standard: Option<String>,
    pub unit: Option<String>,
    pub lower_limit: Option<Decimal>,
    pub upper_limit: Option<Decimal>,
    pub is_critical: bool,
    pub is_active: bool,
}

impl InspectionCharacteristic {
    pub fn new(
        char_id: InspectionCharId,
        char_name: impl Into<String>,
    ) -> Result<Self, MasterDataDomainError> {
        let char_name = char_name.into().trim().to_string();

        if char_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            char_id,
            char_name,
            material_type: None,
            inspection_type: None,
            method: None,
            standard: None,
            unit: None,
            lower_limit: None,
            upper_limit: None,
            is_critical: false,
            is_active: true,
        })
    }

    pub fn set_limits(
        &mut self,
        lower_limit: Option<Decimal>,
        upper_limit: Option<Decimal>,
    ) -> Result<(), MasterDataDomainError> {
        if let (Some(lower), Some(upper)) = (lower_limit, upper_limit) {
            if upper < lower {
                return Err(MasterDataDomainError::InspectionLimitInvalid);
            }
        }

        self.lower_limit = lower_limit;
        self.upper_limit = upper_limit;
        Ok(())
    }

    pub fn rename(&mut self, char_name: impl Into<String>) -> Result<(), MasterDataDomainError> {
        let char_name = char_name.into().trim().to_string();
        if char_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.char_name = char_name;
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectCodeMaster {
    pub defect_code: DefectCode,
    pub defect_name: String,
    pub category: Option<String>,
    pub severity: DefectSeverity,
    pub description: Option<String>,
    pub is_active: bool,
}

impl DefectCodeMaster {
    pub fn new(
        defect_code: DefectCode,
        defect_name: impl Into<String>,
        severity: DefectSeverity,
    ) -> Result<Self, MasterDataDomainError> {
        let defect_name = defect_name.into().trim().to_string();

        if defect_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }

        Ok(Self {
            defect_code,
            defect_name,
            category: None,
            severity,
            description: None,
            is_active: true,
        })
    }

    pub fn rename(&mut self, defect_name: impl Into<String>) -> Result<(), MasterDataDomainError> {
        let defect_name = defect_name.into().trim().to_string();
        if defect_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.defect_name = defect_name;
        Ok(())
    }

    pub fn change_severity(&mut self, severity: DefectSeverity) {
        self.severity = severity;
    }

    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

// ============================================================
// 单元测试 — 计划 §五 / §六 的领域规则
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).expect("test fixture should be valid")
    }

    fn mid(s: &str) -> MaterialId {
        MaterialId::new(s).expect("test fixture should be valid")
    }

    fn binc(s: &str) -> BinCode {
        BinCode::new(s).expect("test fixture should be valid")
    }

    // -------------------- Material --------------------

    #[test]
    fn material_rejects_empty_name() {
        let r = Material::new(
            mid("M001"),
            "   ", // 修剪后空
            MaterialType::RawMaterial,
            "EA",
            "RM",
            0,
            0,
            d("0"),
            d("0"),
        );
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn material_rejects_negative_safety_stock() {
        let r = Material::new(
            mid("M001"),
            "Steel",
            MaterialType::RawMaterial,
            "EA",
            "RM",
            -1,
            0,
            d("10"),
            d("10"),
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::AmountCannotBeNegative)
        ));
    }

    #[test]
    fn material_rejects_negative_price() {
        let r = Material::new(
            mid("M001"),
            "Steel",
            MaterialType::RawMaterial,
            "EA",
            "RM",
            0,
            0,
            d("-1"),
            d("0"),
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::AmountCannotBeNegative)
        ));
    }

    #[test]
    fn material_valid_inputs_succeed() {
        let m = Material::new(
            mid("M001"),
            "Steel Bar",
            MaterialType::RawMaterial,
            "EA",
            "RM",
            10,
            5,
            d("100"),
            d("95.5"),
        )
        .expect("test fixture should be valid");
        assert_eq!(m.material_id.value(), "M001");
        assert_eq!(m.material_name, "Steel Bar");
        assert_eq!(m.current_stock, 0); // 新物料库存初始 0
        assert_eq!(m.status, "正常");
    }

    // -------------------- StorageBin --------------------

    #[test]
    fn bin_rejects_empty_zone_or_type() {
        assert!(StorageBin::new(binc("A1"), " ", "RACK", 100).is_err());
        assert!(StorageBin::new(binc("A1"), "RM", " ", 100).is_err());
    }

    #[test]
    fn bin_rejects_negative_capacity() {
        let r = StorageBin::new(binc("A1"), "RM", "RACK", -1);
        assert!(matches!(
            r,
            Err(MasterDataDomainError::CapacityCannotBeNegative)
        ));
    }

    #[test]
    fn bin_rejects_zero_capacity() {
        let r = StorageBin::new(binc("A1"), "RM", "RACK", 0);
        assert!(matches!(r, Err(MasterDataDomainError::BinCapacityInvalid)));
    }

    #[test]
    fn bin_initial_state() {
        let b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        assert_eq!(b.capacity, 100);
        assert_eq!(b.current_occupied, 0);
        assert_eq!(b.status, "正常");
    }

    #[test]
    fn change_capacity_below_occupied_fails() {
        // 计划 §五.2 / §六.2:容量不能小于当前占用
        let mut b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        b.current_occupied = 80;
        let r = b.change_capacity(50);
        assert!(matches!(
            r,
            Err(MasterDataDomainError::CapacityCannotBeLessThanOccupied)
        ));
        // 失败时 capacity 应保持不变
        assert_eq!(b.capacity, 100);
    }

    #[test]
    fn change_capacity_to_negative_fails() {
        let mut b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        let r = b.change_capacity(-1);
        assert!(matches!(
            r,
            Err(MasterDataDomainError::CapacityCannotBeNegative)
        ));
    }

    #[test]
    fn change_capacity_to_zero_fails() {
        let mut b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        let r = b.change_capacity(0);
        assert!(matches!(r, Err(MasterDataDomainError::BinCapacityInvalid)));
        assert_eq!(b.capacity, 100);
    }

    #[test]
    fn change_capacity_above_occupied_succeeds() {
        let mut b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        b.current_occupied = 30;
        b.change_capacity(150)
            .expect("test fixture should be valid");
        assert_eq!(b.capacity, 150);
    }

    #[test]
    fn change_capacity_equal_to_occupied_succeeds() {
        // 边界:>= 是允许的
        let mut b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        b.current_occupied = 50;
        b.change_capacity(50).expect("test fixture should be valid");
        assert_eq!(b.capacity, 50);
    }

    #[test]
    fn bin_deactivate_maps_to_db_allowed_frozen_status() {
        let mut b =
            StorageBin::new(binc("A1"), "RM", "RACK", 100).expect("test fixture should be valid");
        b.deactivate();
        assert_eq!(b.status, "冻结");
    }

    // -------------------- ProductVariant --------------------

    #[test]
    fn product_variant_rejects_empty_name() {
        // 计划 §五.6:变体名称非空
        let r = ProductVariant::new(
            VariantCode::new("V001").expect("test fixture should be valid"),
            "  ",
            mid("M001"),
            d("100"),
        );
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn product_variant_rejects_negative_cost() {
        let r = ProductVariant::new(
            VariantCode::new("V001").expect("test fixture should be valid"),
            "Standard",
            mid("M001"),
            d("-1"),
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::AmountCannotBeNegative)
        ));
    }

    #[test]
    fn product_variant_default_state() {
        let v = ProductVariant::new(
            VariantCode::new("V001").expect("test fixture should be valid"),
            "Standard",
            mid("M001"),
            d("99.99"),
        )
        .expect("test fixture should be valid");
        assert_eq!(v.standard_cost, d("99.99"));
        assert!(v.bom_id.is_none()); // 默认未绑定 BOM
        assert!(v.is_active);
    }

    // -------------------- BomHeader --------------------

    #[test]
    fn bom_header_rejects_empty_name() {
        let r = BomHeader::new(
            BomId::new("B001").expect("test fixture should be valid"),
            "   ",
            mid("M001"),
            "v1",
        );
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn bom_header_rejects_empty_version() {
        let r = BomHeader::new(
            BomId::new("B001").expect("test fixture should be valid"),
            "Top BOM",
            mid("M001"),
            "  ",
        );
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn bom_header_initial_state_is_draft() {
        // 计划 §五.7:新建 BOM 默认草稿状态(由 entity 强制),
        // 启用必须经 activate_bom 端点(那里跑组件数 + 循环引用前置)。
        let h = BomHeader::new(
            BomId::new("B001").expect("test fixture should be valid"),
            "Top BOM",
            mid("M001"),
            "v1",
        )
        .expect("test fixture should be valid");
        assert!(matches!(h.status, BomStatus::Draft));
        assert!(h.is_active); // 注:这里 is_active 是 entity 默认 true,但 status 是 Draft;
        // 实际启用流程要靠 activate_bom 修改 status='生效'
        assert!(h.variant_code.is_none());
    }

    // -------------------- BomComponent --------------------

    #[test]
    fn bom_component_rejects_self_reference() {
        // 计划 §五 / §六:BOM 禁止自引用
        let m = mid("M100");
        let r = BomComponent::new(
            BomId::new("BOM01").expect("test fixture should be valid"),
            m.clone(),
            m.clone(),
            d("1"),
            "EA",
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::BomComponentCannotReferenceItself)
        ));
    }

    #[test]
    fn bom_component_rejects_zero_or_negative_quantity() {
        // 计划 §五 / §六:组件数量必须大于 0
        let parent = mid("P");
        let child = mid("C");
        for q in [d("0"), d("-1"), d("-0.001")] {
            let r = BomComponent::new(
                BomId::new("B").expect("test fixture should be valid"),
                parent.clone(),
                child.clone(),
                q,
                "EA",
            );
            assert!(matches!(
                r,
                Err(MasterDataDomainError::QuantityMustBeGreaterThanZero)
            ));
        }
    }

    #[test]
    fn bom_component_valid_inputs_succeed() {
        let c = BomComponent::new(
            BomId::new("B1").expect("test fixture should be valid"),
            mid("P1"),
            mid("C1"),
            d("2.5"),
            "EA",
        )
        .expect("test fixture should be valid");
        assert_eq!(c.quantity, d("2.5"));
        assert_eq!(c.unit, "EA");
        assert_eq!(c.bom_level, 1);
    }

    // -------------------- Supplier --------------------

    #[test]
    fn supplier_rejects_empty_name() {
        let r = Supplier::new(
            SupplierId::new("S001").expect("test fixture should be valid"),
            "  ",
        );
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn supplier_default_state() {
        let s = Supplier::new(
            SupplierId::new("S001").expect("test fixture should be valid"),
            "ACME",
        )
        .expect("test fixture should be valid");
        assert_eq!(s.supplier_name, "ACME");
        assert_eq!(s.quality_rating, "A");
        assert!(s.is_active);
        assert!(s.contact_person.is_none());
    }

    #[test]
    fn supplier_trims_name() {
        let s = Supplier::new(
            SupplierId::new("S001").expect("test fixture should be valid"),
            "  ACME  ",
        )
        .expect("test fixture should be valid");
        assert_eq!(s.supplier_name, "ACME");
    }

    // -------------------- Customer --------------------

    #[test]
    fn customer_rejects_empty_name() {
        // 计划 §五.5:客户名称不能为空
        let r = Customer::new(
            CustomerId::new("C001").expect("test fixture should be valid"),
            " ",
        );
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn customer_default_state() {
        let c = Customer::new(
            CustomerId::new("C001").expect("test fixture should be valid"),
            "Beta Corp",
        )
        .expect("test fixture should be valid");
        assert_eq!(c.customer_name, "Beta Corp");
        assert_eq!(c.credit_limit, Decimal::ZERO);
        assert!(c.is_active);
    }

    #[test]
    fn customer_rejects_negative_credit_limit() {
        let mut c = Customer::new(
            CustomerId::new("C001").expect("test fixture should be valid"),
            "Beta Corp",
        )
        .expect("test fixture should be valid");
        let r = c.change_credit_limit(d("-0.01"));
        assert!(matches!(
            r,
            Err(MasterDataDomainError::AmountCannotBeNegative)
        ));
        assert_eq!(c.credit_limit, Decimal::ZERO);
    }

    // -------------------- MaterialSupplier --------------------

    #[test]
    fn material_supplier_rejects_negative_lead_time() {
        // 计划 §五.4:采购提前期不能小于 0
        let r = MaterialSupplier::new(
            mid("M001"),
            SupplierId::new("S001").expect("test fixture should be valid"),
            false,
            -1,
            1,
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::AmountCannotBeNegative)
        ));
    }

    #[test]
    fn material_supplier_rejects_zero_moq() {
        // 执行约定 v1:最小采购量必须大于等于 1
        let r = MaterialSupplier::new(
            mid("M001"),
            SupplierId::new("S001").expect("test fixture should be valid"),
            false,
            7,
            0,
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::QuantityMustBeGreaterThanZero)
        ));
    }

    #[test]
    fn material_supplier_change_moq_rejects_zero() {
        let mut ms = MaterialSupplier::new(
            mid("M001"),
            SupplierId::new("S001").expect("test fixture should be valid"),
            false,
            7,
            1,
        )
        .expect("test fixture should be valid");
        let r = ms.change_moq(0);
        assert!(matches!(
            r,
            Err(MasterDataDomainError::QuantityMustBeGreaterThanZero)
        ));
        assert_eq!(ms.moq, 1);
    }

    #[test]
    fn material_supplier_valid_inputs_succeed() {
        let ms = MaterialSupplier::new(
            mid("M001"),
            SupplierId::new("S001").expect("test fixture should be valid"),
            true,
            14,
            10,
        )
        .expect("test fixture should be valid");
        assert_eq!(ms.lead_time_days, 14);
        assert_eq!(ms.moq, 10);
        assert!(ms.is_primary);
        assert!(ms.is_active);
        assert!(ms.purchase_price.is_none());
    }

    // -------------------- WorkCenter --------------------

    #[test]
    fn work_center_rejects_zero_capacity() {
        let mut wc = WorkCenter::new(
            WorkCenterId::new("WC001").expect("test fixture should be valid"),
            "Cutting",
        )
        .expect("test fixture should be valid");
        let r = wc.change_capacity(Some(0));
        assert!(matches!(
            r,
            Err(MasterDataDomainError::QuantityMustBeGreaterThanZero)
        ));
        assert_eq!(wc.capacity_per_day, None);
    }

    #[test]
    fn work_center_rejects_zero_efficiency() {
        let mut wc = WorkCenter::new(
            WorkCenterId::new("WC001").expect("test fixture should be valid"),
            "Cutting",
        )
        .expect("test fixture should be valid");
        let r = wc.change_efficiency(Decimal::ZERO);
        assert!(matches!(
            r,
            Err(MasterDataDomainError::AmountCannotBeNegative)
        ));
        assert_eq!(wc.efficiency, Decimal::new(10000, 2));
    }

    // -------------------- InspectionCharacteristic --------------------

    #[test]
    fn inspection_limits_inverted_fails() {
        // 计划 §五 / §六:检验上下限合法 — upper >= lower
        let mut ic = InspectionCharacteristic::new(
            InspectionCharId::new("IC001").expect("test fixture should be valid"),
            "Length",
        )
        .expect("test fixture should be valid");
        let r = ic.set_limits(Some(d("10")), Some(d("5")));
        assert!(matches!(
            r,
            Err(MasterDataDomainError::InspectionLimitInvalid)
        ));
    }

    #[test]
    fn inspection_limits_equal_succeeds() {
        let mut ic = InspectionCharacteristic::new(
            InspectionCharId::new("IC001").expect("test fixture should be valid"),
            "Length",
        )
        .expect("test fixture should be valid");
        ic.set_limits(Some(d("5")), Some(d("5")))
            .expect("test fixture should be valid");
        assert_eq!(ic.lower_limit, Some(d("5")));
        assert_eq!(ic.upper_limit, Some(d("5")));
    }

    #[test]
    fn inspection_one_sided_limits_succeed() {
        // 仅一侧上限或下限,无 lower-vs-upper 比较问题
        let mut ic = InspectionCharacteristic::new(
            InspectionCharId::new("IC001").expect("test fixture should be valid"),
            "Length",
        )
        .expect("test fixture should be valid");
        ic.set_limits(None, Some(d("100")))
            .expect("test fixture should be valid");
        ic.set_limits(Some(d("0")), None)
            .expect("test fixture should be valid");
    }
}
