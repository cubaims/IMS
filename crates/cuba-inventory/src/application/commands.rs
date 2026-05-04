use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
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

    #[validate(range(min = 1))]
    pub quantity: i32,

    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub quality_status: Option<String>,
    pub remark: Option<String>,
    pub unit_price: Option<Decimal>,
    pub posting_date: Option<DateTime<Utc>>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TransferInventoryCommand {
    #[validate(length(min = 1))]
    pub material_id: String,

    #[validate(range(min = 1))]
    pub quantity: i32,

    #[validate(length(min = 1))]
    pub from_bin: String,

    #[validate(length(min = 1))]
    pub to_bin: String,

    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub quality_status: Option<String>,
    pub remark: Option<String>,
    pub posting_date: Option<DateTime<Utc>>,
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

    #[validate(range(min = 1))]
    pub quantity: i32,

    pub from_zone: Option<String>,
    pub quality_status: Option<String>,
}
