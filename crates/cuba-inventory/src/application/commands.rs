use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use validator::Validate;

use crate::domain::{
    BatchNumber, BinCode, InventoryPosting, MaterialId, MovementType, QualityStatus, Quantity,
};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PostInventoryCommand {
    #[validate(length(min = 1))]
    pub material_id: String,

    #[validate(length(min = 3, max = 3))]
    pub movement_type: String,

    pub quantity: Decimal,

    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub quality_status: Option<String>,
    pub remark: Option<String>,
    pub unit_price: Option<Decimal>,
    pub posting_date: Option<OffsetDateTime>,
}

impl PostInventoryCommand {
    pub fn to_domain(&self) -> Result<InventoryPosting, String> {
        let movement_type =
            MovementType::from_str(&self.movement_type).map_err(|err| err.to_string())?;

        let quality_status = match &self.quality_status {
            Some(value) => QualityStatus::from_str(value).map_err(|err| err.to_string())?,
            None => QualityStatus::Qualified,
        };

        let posting = InventoryPosting {
            material_id: MaterialId::new(self.material_id.clone())
                .map_err(|err| err.to_string())?,
            movement_type,
            quantity: Quantity::new(self.quantity).map_err(|err| err.to_string())?,
            from_bin: self
                .from_bin
                .clone()
                .map(BinCode::new)
                .transpose()
                .map_err(|err| err.to_string())?,
            to_bin: self
                .to_bin
                .clone()
                .map(BinCode::new)
                .transpose()
                .map_err(|err| err.to_string())?,
            batch_number: self
                .batch_number
                .clone()
                .map(BatchNumber::new)
                .transpose()
                .map_err(|err| err.to_string())?,
            serial_number: self.serial_number.clone(),
            reference_doc: self.reference_doc.clone(),
            remark: self.remark.clone(),
            quality_status,
            unit_price: self.unit_price,
        };

        posting.validate().map_err(|err| err.to_string())?;

        Ok(posting)
    }

    pub fn validate_manual_posting_type(&self) -> Result<(), String> {
        let movement_type =
            MovementType::from_str(&self.movement_type).map_err(|err| err.to_string())?;

        if movement_type.is_manual_posting() {
            Ok(())
        } else {
            Err("movement type 311 must use the transfer endpoint".to_string())
        }
    }

    pub fn quantity_as_i32(&self) -> Result<i32, String> {
        Quantity::new(self.quantity)
            .and_then(Quantity::to_i32)
            .map_err(|err| err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TransferInventoryCommand {
    #[validate(length(min = 1))]
    pub material_id: String,

    pub quantity: Decimal,

    #[validate(length(min = 1))]
    pub from_bin: String,

    #[validate(length(min = 1))]
    pub to_bin: String,

    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub quality_status: Option<String>,
    pub remark: Option<String>,
    pub posting_date: Option<OffsetDateTime>,
}

impl TransferInventoryCommand {
    pub fn into_post_command(self) -> PostInventoryCommand {
        PostInventoryCommand {
            material_id: self.material_id,
            movement_type: "311".to_string(),
            quantity: self.quantity,
            from_bin: Some(self.from_bin),
            to_bin: Some(self.to_bin),
            batch_number: self.batch_number,
            serial_number: self.serial_number,
            reference_doc: self.reference_doc,
            quality_status: self.quality_status,
            remark: self.remark,
            unit_price: None,
            posting_date: self.posting_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PickBatchFefoCommand {
    #[validate(length(min = 1))]
    pub material_id: String,

    pub quantity: Decimal,

    pub from_zone: Option<String>,
    pub quality_status: Option<String>,
}

impl PickBatchFefoCommand {
    pub fn quantity_as_i32(&self) -> Result<i32, String> {
        Quantity::new(self.quantity)
            .and_then(Quantity::to_i32)
            .map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post_command(movement_type: &str) -> PostInventoryCommand {
        PostInventoryCommand {
            material_id: "RM001".to_string(),
            movement_type: movement_type.to_string(),
            quantity: Decimal::ONE,
            from_bin: None,
            to_bin: Some("RM-A01".to_string()),
            batch_number: Some("BATCH-001".to_string()),
            serial_number: None,
            reference_doc: Some("MANUAL-001".to_string()),
            quality_status: Some("合格".to_string()),
            remark: None,
            unit_price: None,
            posting_date: None,
        }
    }

    #[test]
    fn manual_posting_type_rejects_transfer_311() {
        let command = post_command("311");

        assert_eq!(
            command.validate_manual_posting_type(),
            Err("movement type 311 must use the transfer endpoint".to_string())
        );
    }

    #[test]
    fn manual_posting_type_accepts_non_transfer_movements() {
        for movement_type in ["101", "261", "701", "702", "999"] {
            let command = post_command(movement_type);

            assert!(command.validate_manual_posting_type().is_ok());
        }
    }
}
