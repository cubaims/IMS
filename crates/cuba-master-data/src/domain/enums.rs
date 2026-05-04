use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialType {
    RawMaterial,
    SemiFinished,
    FinishedGoods,
}

impl MaterialType {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::RawMaterial => "原材料",
            Self::SemiFinished => "半成品",
            Self::FinishedGoods => "成品",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveStatus {
    Active,
    Inactive,
}

impl ActiveStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinStatus {
    Normal,
    Occupied,
    Maintenance,
    Frozen,
}

impl BinStatus {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Occupied => "占用",
            Self::Maintenance => "维护中",
            Self::Frozen => "冻结",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BomStatus {
    Draft,
    Active,
    Inactive,
}

impl BomStatus {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Active => "生效",
            Self::Inactive => "失效",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefectSeverity {
    Minor,
    Major,
    Critical,
}

impl DefectSeverity {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::Minor => "一般",
            Self::Major => "严重",
            Self::Critical => "紧急",
        }
    }
}