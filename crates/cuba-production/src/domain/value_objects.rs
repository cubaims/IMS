use serde::{Deserialize, Serialize};
use std::fmt;

use super::ProductionDomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductionOrderId(pub String);

impl ProductionOrderId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProductionOrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(pub String);

impl MaterialId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BomId(pub String);

impl BomId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantCode(pub String);

impl VariantCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkCenterId(pub String);

impl WorkCenterId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchNumber(pub String);

impl BatchNumber {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionQuantity(pub i32);

impl ProductionQuantity {
    pub fn new(value: i32) -> Result<Self, ProductionDomainError> {
        if value <= 0 {
            return Err(ProductionDomainError::InvalidPlannedQuantity);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductionOrderStatus {
    Draft,
    Planned,
    Released,
    PartiallyCompleted,
    Completed,
    Closed,
    Cancelled,
}

impl ProductionOrderStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Planned => "PLANNED",
            Self::Released => "RELEASED",
            Self::PartiallyCompleted => "PARTIALLY_COMPLETED",
            Self::Completed => "COMPLETED",
            Self::Closed => "CLOSED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn can_release(self) -> bool {
        matches!(self, Self::Planned)
    }

    pub fn can_complete(self) -> bool {
        matches!(self, Self::Released | Self::PartiallyCompleted)
    }

    pub fn can_cancel(self) -> bool {
        matches!(self, Self::Draft | Self::Planned | Self::Released)
    }

    pub fn can_close(self) -> bool {
        matches!(self, Self::Completed)
    }
}

impl TryFrom<&str> for ProductionOrderStatus {
    type Error = ProductionDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DRAFT" => Ok(Self::Draft),
            "PLANNED" => Ok(Self::Planned),
            "RELEASED" => Ok(Self::Released),
            "PARTIALLY_COMPLETED" => Ok(Self::PartiallyCompleted),
            "COMPLETED" => Ok(Self::Completed),
            "CLOSED" => Ok(Self::Closed),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(ProductionDomainError::ProductionOrderStatusInvalid(
                other.to_string(),
            )),
        }
    }
}