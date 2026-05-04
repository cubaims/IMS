use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransaction {
    pub transaction_id: TransactionId,
    pub material_id: MaterialId,
    pub movement_type: MovementType,
    pub quantity: i32,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub operator: Option<String>,
    pub transaction_date: DateTime<Utc>,
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
    pub qty: i32,
    pub serial_count: i32,
    pub last_transaction_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinStock {
    pub material_id: String,
    pub bin_code: String,
    pub batch_number: Option<String>,
    pub quality_status: String,
    pub qty: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub batch_number: String,
    pub material_id: String,
    pub production_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub quality_grade: Option<String>,
    pub current_stock: i32,
    pub current_bin: Option<String>,
    pub quality_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub old_stock: Option<i32>,
    pub new_stock: Option<i32>,
    pub transaction_id: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: DateTime<Utc>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapHistory {
    pub history_id: i64,
    pub material_id: String,
    pub old_map_price: Decimal,
    pub new_map_price: Decimal,
    pub old_stock_qty: i32,
    pub new_stock_qty: i32,
    pub incoming_qty: i32,
    pub incoming_unit_price: Decimal,
    pub transaction_id: Option<String>,
    pub calculation_formula: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryPostingResult {
    pub transaction_id: String,
    pub material_id: String,
    pub movement_type: String,
    pub quantity: i32,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub reference_doc: Option<String>,
    pub map_updated: bool,
}