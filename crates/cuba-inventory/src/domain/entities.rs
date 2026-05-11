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

#[cfg(test)]
mod tests {
    use super::*;

    fn material_id() -> MaterialId {
        MaterialId::new("RM001").expect("test fixture should be valid")
    }

    fn quantity() -> Quantity {
        Quantity::from_i32(10).expect("test fixture should be valid")
    }

    fn bin(value: &str) -> BinCode {
        BinCode::new(value).expect("test fixture should be valid")
    }

    fn posting(movement_type: MovementType) -> InventoryPosting {
        InventoryPosting {
            material_id: material_id(),
            movement_type,
            quantity: quantity(),
            from_bin: None,
            to_bin: None,
            batch_number: None,
            serial_number: None,
            reference_doc: Some("MANUAL-001".to_string()),
            remark: None,
            quality_status: QualityStatus::Qualified,
            unit_price: None,
        }
    }

    #[test]
    fn receipt_101_requires_to_bin_and_rejects_from_bin() {
        let missing_to_bin = posting(MovementType::Receipt101);

        assert!(matches!(
            missing_to_bin.validate(),
            Err(InventoryDomainError::ToBinRequired(movement)) if movement == "101"
        ));

        let mut with_source = posting(MovementType::Receipt101);
        with_source.from_bin = Some(bin("RM-A01"));
        with_source.to_bin = Some(bin("RM-A02"));

        assert!(matches!(
            with_source.validate(),
            Err(InventoryDomainError::FromBinMustBeEmpty(movement)) if movement == "101"
        ));
    }

    #[test]
    fn outbound_movements_require_from_bin_and_reject_to_bin() {
        for movement_type in [
            MovementType::Issue261,
            MovementType::CountLoss702,
            MovementType::Scrap999,
        ] {
            let missing_from_bin = posting(movement_type);
            assert!(matches!(
                missing_from_bin.validate(),
                Err(InventoryDomainError::FromBinRequired(_))
            ));

            let mut with_target = posting(movement_type);
            with_target.from_bin = Some(bin("RM-A01"));
            with_target.to_bin = Some(bin("RM-A02"));
            assert!(matches!(
                with_target.validate(),
                Err(InventoryDomainError::ToBinMustBeEmpty(_))
            ));
        }
    }

    #[test]
    fn transfer_311_requires_distinct_source_and_target_bins() {
        let missing_bins = posting(MovementType::Transfer311);
        assert!(matches!(
            missing_bins.validate(),
            Err(InventoryDomainError::FromBinRequired(movement)) if movement == "311"
        ));

        let mut same_bins = posting(MovementType::Transfer311);
        same_bins.from_bin = Some(bin("RM-A01"));
        same_bins.to_bin = Some(bin("RM-A01"));
        assert!(matches!(
            same_bins.validate(),
            Err(InventoryDomainError::SameSourceAndTargetBin)
        ));

        let mut valid = posting(MovementType::Transfer311);
        valid.from_bin = Some(bin("RM-A01"));
        valid.to_bin = Some(bin("RM-A02"));
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn manual_posting_requires_reference_doc_or_remark() {
        let mut command = posting(MovementType::Receipt101);
        command.to_bin = Some(bin("RM-A01"));
        command.reference_doc = Some("   ".to_string());
        command.remark = None;

        assert!(matches!(
            command.validate(),
            Err(InventoryDomainError::ReferenceDocOrRemarkRequired)
        ));

        command.remark = Some("manual receipt".to_string());
        assert!(command.validate().is_ok());
    }

    #[test]
    fn outbound_movements_reject_frozen_or_scrapped_quality_status() {
        for quality_status in [QualityStatus::Frozen, QualityStatus::Scrapped] {
            let mut command = posting(MovementType::Issue261);
            command.from_bin = Some(bin("RM-A01"));
            command.quality_status = quality_status;

            assert!(matches!(
                command.validate(),
                Err(InventoryDomainError::InvalidOutboundQualityStatus(_))
            ));
        }
    }

    #[test]
    fn unit_price_must_be_positive_when_present() {
        let mut command = posting(MovementType::Receipt101);
        command.to_bin = Some(bin("RM-A01"));
        command.unit_price = Some(Decimal::ZERO);

        assert!(matches!(
            command.validate(),
            Err(InventoryDomainError::InvalidUnitPrice)
        ));
    }
}
