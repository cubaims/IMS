use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PurchaseOrderId(pub String);

impl PurchaseOrderId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseLineNo(pub i32);

impl PurchaseLineNo {
    pub fn new(value: i32) -> Result<Self, crate::domain::PurchaseDomainError> {
        if value <= 0 {
            return Err(crate::domain::PurchaseDomainError::InvalidLineNo);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseQuantity(pub i32);

impl PurchaseQuantity {
    pub fn new(value: i32) -> Result<Self, crate::domain::PurchaseDomainError> {
        if value <= 0 {
            return Err(crate::domain::PurchaseDomainError::InvalidQuantity);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> i32 {
        self.0
    }
}
