use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SalesOrderId(pub String);

impl SalesOrderId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesLineNo(pub i32);

impl SalesLineNo {
    pub fn new(value: i32) -> Result<Self, crate::domain::SalesDomainError> {
        if value <= 0 {
            return Err(crate::domain::SalesDomainError::InvalidLineNo);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesQuantity(pub i32);

impl SalesQuantity {
    pub fn new(value: i32) -> Result<Self, crate::domain::SalesDomainError> {
        if value <= 0 {
            return Err(crate::domain::SalesDomainError::InvalidQuantity);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> i32 {
        self.0
    }
}
