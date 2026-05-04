use serde::{Deserialize, Serialize};

use super::InventoryDomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(String);

impl MaterialId {
    pub fn new(value: impl Into<String>) -> Result<Self, InventoryDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(InventoryDomainError::EmptyMaterialId);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BinCode(String);

impl BinCode {
    pub fn new(value: impl Into<String>) -> Result<Self, InventoryDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(InventoryDomainError::EmptyBinCode);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchNumber(String);

impl BatchNumber {
    pub fn new(value: impl Into<String>) -> Result<Self, InventoryDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(InventoryDomainError::EmptyBatchNumber);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(String);

impl TransactionId {
    pub fn new(value: impl Into<String>) -> Result<Self, InventoryDomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(InventoryDomainError::EmptyTransactionId);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity(i32);

impl Quantity {
    pub fn new(value: i32) -> Result<Self, InventoryDomainError> {
        if value <= 0 {
            return Err(InventoryDomainError::InvalidQuantity);
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}