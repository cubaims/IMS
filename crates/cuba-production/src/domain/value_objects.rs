use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductionOrderId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BomId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantCode(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkCenterId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchNumber(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BinCode(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductionOrderStatus {
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
            Self::Planned => "计划中",
            Self::Released => "已下达",
            Self::PartiallyCompleted => "生产中",
            Self::Completed => "完成",
            Self::Closed => "关闭",
            Self::Cancelled => "取消",
        }
    }

    pub fn from_db_text(value: &str) -> Self {
        Self::from_api_or_db_text(value).unwrap_or(Self::Planned)
    }

    pub fn from_api_or_db_text(value: &str) -> Option<Self> {
        match value {
            "PLANNED" | "Planned" | "计划中" => Some(Self::Planned),
            "RELEASED" | "Released" | "已下达" => Some(Self::Released),
            "PARTIALLY_COMPLETED" | "PartiallyCompleted" | "生产中" => {
                Some(Self::PartiallyCompleted)
            }
            "COMPLETED" | "Completed" | "完成" => Some(Self::Completed),
            "CLOSED" | "Closed" | "关闭" => Some(Self::Closed),
            "CANCELLED" | "Cancelled" | "取消" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn as_api_code(self) -> &'static str {
        match self {
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

    pub fn ensure_can_release(self) -> Result<(), crate::domain::ProductionDomainError> {
        if self.can_release() {
            Ok(())
        } else {
            Err(crate::domain::ProductionDomainError::ProductionOrderStatusInvalid)
        }
    }

    pub fn can_complete(self) -> bool {
        matches!(self, Self::Released | Self::PartiallyCompleted)
    }

    pub fn ensure_can_complete(self) -> Result<(), crate::domain::ProductionDomainError> {
        if self.can_complete() {
            Ok(())
        } else {
            Err(crate::domain::ProductionDomainError::ProductionOrderStatusInvalid)
        }
    }

    pub fn can_cancel(self) -> bool {
        matches!(self, Self::Planned | Self::Released)
    }

    pub fn ensure_can_cancel(self) -> Result<(), crate::domain::ProductionDomainError> {
        if self.can_cancel() {
            Ok(())
        } else {
            Err(crate::domain::ProductionDomainError::ProductionOrderStatusInvalid)
        }
    }

    pub fn can_close(self) -> bool {
        matches!(self, Self::Completed)
    }

    pub fn ensure_can_close(self) -> Result<(), crate::domain::ProductionDomainError> {
        if self.can_close() {
            Ok(())
        } else {
            Err(crate::domain::ProductionDomainError::ProductionOrderStatusInvalid)
        }
    }

    pub fn can_update_plan(self) -> bool {
        matches!(self, Self::Planned)
    }

    pub fn ensure_can_update_plan(self) -> Result<(), crate::domain::ProductionDomainError> {
        if self.can_update_plan() {
            Ok(())
        } else {
            Err(crate::domain::ProductionDomainError::ProductionOrderStatusInvalid)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickStrategy {
    Fefo,
    Manual,
}

impl Default for PickStrategy {
    fn default() -> Self {
        Self::Fefo
    }
}
