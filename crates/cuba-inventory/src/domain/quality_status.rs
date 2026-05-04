use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use super::InventoryDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QualityStatus {
    Pending,
    #[default]
    Qualified,
    Frozen,
    Scrapped,
}

impl QualityStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Pending => "待检",
            Self::Qualified => "合格",
            Self::Frozen => "冻结",
            Self::Scrapped => "报废",
        }
    }

    pub fn can_issue(self) -> bool {
        matches!(self, Self::Qualified)
    }
}


impl fmt::Display for QualityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_text())
    }
}

impl FromStr for QualityStatus {
    type Err = InventoryDomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "待检" | "pending" | "Pending" => Ok(Self::Pending),
            "合格" | "qualified" | "Qualified" => Ok(Self::Qualified),
            "冻结" | "frozen" | "Frozen" => Ok(Self::Frozen),
            "报废" | "scrapped" | "Scrapped" => Ok(Self::Scrapped),
            other => Err(InventoryDomainError::InvalidQualityStatus(
                other.to_string(),
            )),
        }
    }
}
