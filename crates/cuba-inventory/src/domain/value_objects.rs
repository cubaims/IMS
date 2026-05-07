use rust_decimal::Decimal;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Quantity(Decimal);

impl Quantity {
    pub fn new(value: Decimal) -> Result<Self, InventoryDomainError> {
        if value <= Decimal::ZERO {
            return Err(InventoryDomainError::InvalidQuantity);
        }

        Ok(Self(value))
    }

    pub fn from_i32(value: i32) -> Result<Self, InventoryDomainError> {
        Self::new(Decimal::from(value))
    }

    pub fn value(&self) -> Decimal {
        self.0
    }

    pub fn into_inner(self) -> Decimal {
        self.0
    }
}
