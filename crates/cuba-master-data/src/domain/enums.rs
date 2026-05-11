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

    /// 反向解析 DB 值/中文枚举值。
    /// 同时容忍英文别名(RAW / RAW_MATERIAL / SEMI / SEMI_FINISHED /
    /// FINISHED / FINISHED_GOODS)以备前端传入。
    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "原材料" | "RAW" | "RAW_MATERIAL" => Some(Self::RawMaterial),
            "半成品" | "SEMI" | "SEMI_FINISHED" => Some(Self::SemiFinished),
            "成品" | "FINISHED" | "FINISHED_GOODS" => Some(Self::FinishedGoods),
            _ => None,
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

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "正常" => Some(Self::Normal),
            "占用" => Some(Self::Occupied),
            "维护中" => Some(Self::Maintenance),
            "冻结" => Some(Self::Frozen),
            _ => None,
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

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "草稿" | "DRAFT" => Some(Self::Draft),
            "生效" | "ACTIVE" => Some(Self::Active),
            "失效" | "INACTIVE" => Some(Self::Inactive),
            _ => None,
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

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "一般" | "MINOR" => Some(Self::Minor),
            "严重" | "MAJOR" => Some(Self::Major),
            "紧急" | "CRITICAL" => Some(Self::Critical),
            _ => None,
        }
    }
}

// ============================================================
// 单元测试 — 计划 §五 各业务实体的"领域规则"小节,枚举层验收
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_type_round_trip() {
        for (variant, cn) in [
            (MaterialType::RawMaterial, "原材料"),
            (MaterialType::SemiFinished, "半成品"),
            (MaterialType::FinishedGoods, "成品"),
        ] {
            assert_eq!(variant.as_db_value(), cn);
            assert_eq!(MaterialType::from_db_value(cn), Some(variant));
        }
    }

    #[test]
    fn material_type_accepts_english_alias() {
        assert_eq!(
            MaterialType::from_db_value("RAW_MATERIAL"),
            Some(MaterialType::RawMaterial)
        );
        assert_eq!(
            MaterialType::from_db_value("RAW"),
            Some(MaterialType::RawMaterial)
        );
        assert_eq!(
            MaterialType::from_db_value("SEMI"),
            Some(MaterialType::SemiFinished)
        );
        assert_eq!(
            MaterialType::from_db_value("FINISHED_GOODS"),
            Some(MaterialType::FinishedGoods)
        );
        assert_eq!(
            MaterialType::from_db_value("FINISHED"),
            Some(MaterialType::FinishedGoods)
        );
    }

    #[test]
    fn material_type_unknown_returns_none() {
        assert_eq!(MaterialType::from_db_value(""), None);
        assert_eq!(MaterialType::from_db_value("废料"), None);
    }

    #[test]
    fn bin_status_round_trip() {
        for s in [
            BinStatus::Normal,
            BinStatus::Occupied,
            BinStatus::Maintenance,
            BinStatus::Frozen,
        ] {
            assert_eq!(BinStatus::from_db_value(s.as_db_value()), Some(s));
        }
    }

    #[test]
    fn bom_status_accepts_english_alias() {
        assert_eq!(BomStatus::from_db_value("DRAFT"), Some(BomStatus::Draft));
        assert_eq!(BomStatus::from_db_value("ACTIVE"), Some(BomStatus::Active));
        assert_eq!(
            BomStatus::from_db_value("INACTIVE"),
            Some(BomStatus::Inactive)
        );
    }

    #[test]
    fn defect_severity_accepts_english_alias() {
        assert_eq!(
            DefectSeverity::from_db_value("MINOR"),
            Some(DefectSeverity::Minor)
        );
        assert_eq!(
            DefectSeverity::from_db_value("MAJOR"),
            Some(DefectSeverity::Major)
        );
        assert_eq!(
            DefectSeverity::from_db_value("CRITICAL"),
            Some(DefectSeverity::Critical)
        );
    }
}
