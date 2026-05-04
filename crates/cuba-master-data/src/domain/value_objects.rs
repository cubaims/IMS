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
