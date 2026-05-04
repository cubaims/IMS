use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use super::InventoryDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementType {
    Receipt101,
    Issue261,
    Transfer311,
    CountGain701,
    CountLoss702,
    Scrap999,
}

impl MovementType {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Receipt101 => "101",
            Self::Issue261 => "261",
            Self::Transfer311 => "311",
            Self::CountGain701 => "701",
            Self::CountLoss702 => "702",
            Self::Scrap999 => "999",
        }
    }

    pub fn increases_stock(self) -> bool {
        matches!(self, Self::Receipt101 | Self::CountGain701)
    }

    pub fn decreases_stock(self) -> bool {
        matches!(self, Self::Issue261 | Self::CountLoss702 | Self::Scrap999)
    }

    pub fn is_transfer(self) -> bool {
        matches!(self, Self::Transfer311)
    }

    pub fn requires_from_bin(self) -> bool {
        matches!(
            self,
            Self::Issue261 | Self::Transfer311 | Self::CountLoss702 | Self::Scrap999
        )
    }

    pub fn requires_to_bin(self) -> bool {
        matches!(
            self,
            Self::Receipt101 | Self::Transfer311 | Self::CountGain701
        )
    }

    pub fn can_update_map(self) -> bool {
        matches!(self, Self::Receipt101 | Self::CountGain701)
    }
}

impl fmt::Display for MovementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_code())
    }
}

impl FromStr for MovementType {
    type Err = InventoryDomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "101" => Ok(Self::Receipt101),
            "261" => Ok(Self::Issue261),
            "311" => Ok(Self::Transfer311),
            "701" => Ok(Self::CountGain701),
            "702" => Ok(Self::CountLoss702),
            "999" => Ok(Self::Scrap999),
            other => Err(InventoryDomainError::UnsupportedMovementType(
                other.to_string(),
            )),
        }
    }
}
