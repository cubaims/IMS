use serde::{Deserialize, Serialize};

use super::MasterDataDomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(String);

impl MaterialId {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidMaterialId);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::MaterialIdTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BinCode(String);

impl BinCode {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidBinCode);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::BinCodeTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SupplierId(String);

impl SupplierId {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidSupplierId);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::SupplierIdTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerId(String);

impl CustomerId {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidCustomerId);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::CustomerIdTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BomId(String);

impl BomId {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidBomId);
        }

        if value.len() > 30 {
            return Err(MasterDataDomainError::BomIdTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantCode(String);

impl VariantCode {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidVariantCode);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::VariantCodeTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkCenterId(String);

impl WorkCenterId {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidWorkCenterId);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::WorkCenterIdTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InspectionCharId(String);

impl InspectionCharId {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidInspectionCharId);
        }

        if value.len() > 30 {
            return Err(MasterDataDomainError::InspectionCharIdTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DefectCode(String);

impl DefectCode {
    pub fn new(value: impl Into<String>) -> Result<Self, MasterDataDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(MasterDataDomainError::InvalidDefectCode);
        }

        if value.len() > 20 {
            return Err(MasterDataDomainError::DefectCodeTooLong);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

// ============================================================
// 单元测试 — 计划 §五 的 ID/编码不可重复(配合 DB)、长度限制规则
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_id_rejects_empty_and_whitespace() {
        assert!(MaterialId::new("").is_err());
        assert!(MaterialId::new("   ").is_err());
        assert!(MaterialId::new("\t\n").is_err());
    }

    #[test]
    fn material_id_trims_whitespace() {
        let id = MaterialId::new("  M001  ").unwrap();
        assert_eq!(id.value(), "M001");
    }

    #[test]
    fn material_id_enforces_max_length() {
        // 20 字符是边界,刚好通过
        let ok = "M".repeat(20);
        assert!(MaterialId::new(ok).is_ok());

        // 21 字符超限
        let too_long = "M".repeat(21);
        assert!(MaterialId::new(too_long).is_err());
    }

    #[test]
    fn bin_code_rules() {
        assert!(BinCode::new("").is_err());
        assert!(BinCode::new("RM-A01-S03").is_ok());
        assert!(BinCode::new("X".repeat(20)).is_ok());
        assert!(BinCode::new("X".repeat(21)).is_err());
    }

    #[test]
    fn supplier_and_customer_id_have_same_rules() {
        assert!(SupplierId::new("S001").is_ok());
        assert!(CustomerId::new("C001").is_ok());
        assert!(SupplierId::new(" ").is_err());
        assert!(CustomerId::new("X".repeat(21)).is_err());
    }

    #[test]
    fn bom_id_allows_30_chars() {
        // BOM 比其他 ID 多 10 字符余量
        assert!(BomId::new("X".repeat(30)).is_ok());
        assert!(BomId::new("X".repeat(31)).is_err());
    }

    #[test]
    fn inspection_char_id_allows_30_chars() {
        assert!(InspectionCharId::new("X".repeat(30)).is_ok());
        assert!(InspectionCharId::new("X".repeat(31)).is_err());
    }

    #[test]
    fn variant_work_center_defect_have_20_char_limits() {
        assert!(VariantCode::new("X".repeat(20)).is_ok());
        assert!(VariantCode::new("X".repeat(21)).is_err());
        assert!(WorkCenterId::new("X".repeat(20)).is_ok());
        assert!(WorkCenterId::new("X".repeat(21)).is_err());
        assert!(DefectCode::new("X".repeat(20)).is_ok());
        assert!(DefectCode::new("X".repeat(21)).is_err());
    }
}
