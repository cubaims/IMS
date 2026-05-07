use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use super::{
    BatchNumber, BinCode, InventoryDomainError, MaterialId, MovementType, QualityStatus, Quantity,
    TransactionId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryPosting {
    pub material_id: MaterialId,
    pub movement_type: MovementType,
    pub quantity: Quantity,
    pub from_bin: Option<BinCode>,
    pub to_bin: Option<BinCode>,
    pub batch_number: Option<BatchNumber>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub remark: Option<String>,
    pub quality_status: QualityStatus,
    pub unit_price: Option<Decimal>,
}

impl InventoryPosting {
    pub fn validate(&self) -> Result<(), InventoryDomainError> {
        if self.movement_type.requires_from_bin() && self.from_bin.is_none() {
            return Err(InventoryDomainError::FromBinRequired(
                self.movement_type.to_string(),
            ));
        }

        if self.movement_type.requires_to_bin() && self.to_bin.is_none() {
            return Err(InventoryDomainError::ToBinRequired(
                self.movement_type.to_string(),
            ));
        }

        if self.movement_type.increases_stock() && self.from_bin.is_some() {
            return Err(InventoryDomainError::FromBinMustBeEmpty(
                self.movement_type.to_string(),
            ));
        }

        if self.movement_type.decreases_stock() && self.to_bin.is_some() {
            return Err(InventoryDomainError::ToBinMustBeEmpty(
                self.movement_type.to_string(),
            ));
        }

        if let (Some(from_bin), Some(to_bin)) = (&self.from_bin, &self.to_bin) {
            if from_bin.as_str() == to_bin.as_str() {
                return Err(InventoryDomainError::SameSourceAndTargetBin);
            }
        }

        if let Some(unit_price) = self.unit_price {
            if unit_price <= Decimal::ZERO {
                return Err(InventoryDomainError::InvalidUnitPrice);
            }
        }

        if self
            .reference_doc
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
            && self
                .remark
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(InventoryDomainError::ReferenceDocOrRemarkRequired);
        }

        if self.movement_type.decreases_stock() && !self.quality_status.can_issue() {
            return Err(InventoryDomainError::InvalidOutboundQualityStatus(
                self.quality_status.to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransaction {
    pub transaction_id: TransactionId,
    pub material_id: MaterialId,
    pub movement_type: MovementType,
    pub quantity: Decimal,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub operator: Option<String>,
    pub transaction_date: OffsetDateTime,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStock {
    pub material_id: String,
    pub material_name: String,
    pub bin_code: String,
    pub zone: String,
    pub batch_number: Option<String>,
    pub quality_status: String,
    pub qty: Decimal,
    pub serial_count: i32,
    pub last_transaction_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinStock {
    pub material_id: String,
    pub bin_code: String,
    pub batch_number: Option<String>,
    pub quality_status: String,
    pub qty: Decimal,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub batch_number: String,
    pub material_id: String,
    pub production_date: Option<Date>,
    pub expiry_date: Option<Date>,
    pub quality_grade: Option<String>,
    pub current_stock: Decimal,
    pub current_bin: Option<String>,
    pub quality_status: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHistory {
    pub history_id: i64,
    pub batch_number: String,
    pub material_id: String,
    pub event_type: String,
    pub old_quality_status: Option<String>,
    pub new_quality_status: Option<String>,
    pub old_bin: Option<String>,
    pub new_bin: Option<String>,
    pub old_stock: Option<Decimal>,
    pub new_stock: Option<Decimal>,
    pub transaction_id: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: OffsetDateTime,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapHistory {
    pub history_id: i64,
    pub material_id: String,
    pub old_map_price: Decimal,
    pub new_map_price: Decimal,
    pub old_stock_qty: Decimal,
    pub new_stock_qty: Decimal,
    pub incoming_qty: Decimal,
    pub incoming_unit_price: Decimal,
    pub transaction_id: Option<String>,
    pub calculation_formula: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryPostingResult {
    pub transaction_id: String,
    pub material_id: String,
    pub movement_type: String,
    pub quantity: Decimal,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub reference_doc: Option<String>,
    pub map_updated: bool,
}
