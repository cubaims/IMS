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

        if capacity < self.current_occupied {
            return Err(MasterDataDomainError::CapacityCannotBeLessThanOccupied);
        }

        self.capacity = capacity;
        Ok(())
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
        if lead_time_days < 0 || moq < 0 {
            return Err(MasterDataDomainError::AmountCannotBeNegative);
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
}
