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
pub enum ProductionOrderStatus {
    Planned,
    Released,
    InProduction,
    Completed,
    Cancelled,
}

impl ProductionOrderStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Planned => "计划中",
            Self::Released => "已下达",
            Self::InProduction => "生产中",
            Self::Completed => "完成",
            Self::Cancelled => "取消",
        }
    }

    pub fn from_db_text(value: &str) -> Self {
        match value {
            "已下达" => Self::Released,
            "生产中" => Self::InProduction,
            "完成" => Self::Completed,
            "取消" => Self::Cancelled,
            _ => Self::Planned,
        }
    }

    pub fn can_release(self) -> bool {
        matches!(self, Self::Planned)
    }

    pub fn can_complete(self) -> bool {
        matches!(self, Self::Released | Self::InProduction)
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
