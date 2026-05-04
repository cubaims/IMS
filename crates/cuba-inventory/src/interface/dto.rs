use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::application::{
    BatchHistoryQuery, BatchQuery, CurrentStockQuery, InventoryTransactionQuery, MapHistoryQuery,
    PickBatchFefoCommand, PostInventoryCommand, TransferInventoryCommand,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInventoryRequest {
    pub material_id: String,
    pub movement_type: String,
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

impl From<PostInventoryRequest> for PostInventoryCommand {
    fn from(value: PostInventoryRequest) -> Self {
        Self {
            material_id: value.material_id,
            movement_type: value.movement_type,
            quantity: value.quantity,
            from_bin: value.from_bin,
            to_bin: value.to_bin,
            batch_number: value.batch_number,
            serial_number: value.serial_number,
            reference_doc: value.reference_doc,
            quality_status: value.quality_status,
            remark: value.remark,
            unit_price: value.unit_price,
            posting_date: value.posting_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferInventoryRequest {
    pub material_id: String,
    pub quantity: i32,
    pub from_bin: String,
    pub to_bin: String,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub quality_status: Option<String>,
    pub remark: Option<String>,
    pub posting_date: Option<DateTime<Utc>>,
}

impl From<TransferInventoryRequest> for TransferInventoryCommand {
    fn from(value: TransferInventoryRequest) -> Self {
        Self {
            material_id: value.material_id,
            quantity: value.quantity,
            from_bin: value.from_bin,
            to_bin: value.to_bin,
            batch_number: value.batch_number,
            serial_number: value.serial_number,
            reference_doc: value.reference_doc,
            quality_status: value.quality_status,
            remark: value.remark,
            posting_date: value.posting_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickBatchFefoRequest {
    pub material_id: String,
    pub quantity: i32,
    pub from_zone: Option<String>,
    pub quality_status: Option<String>,
}

impl From<PickBatchFefoRequest> for PickBatchFefoCommand {
    fn from(value: PickBatchFefoRequest) -> Self {
        Self {
            material_id: value.material_id,
            quantity: value.quantity,
            from_zone: value.from_zone,
            quality_status: value.quality_status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrentStockRequest {
    pub material_id: Option<String>,
    pub bin_code: Option<String>,
    pub batch_number: Option<String>,
    pub zone: Option<String>,
    pub quality_status: Option<String>,
    pub only_available: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl From<CurrentStockRequest> for CurrentStockQuery {
    fn from(value: CurrentStockRequest) -> Self {
        Self {
            material_id: value.material_id,
            bin_code: value.bin_code,
            batch_number: value.batch_number,
            zone: value.zone,
            quality_status: value.quality_status,
            only_available: value.only_available,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventoryTransactionRequest {
    pub transaction_id: Option<String>,
    pub material_id: Option<String>,
    pub movement_type: Option<String>,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub reference_doc: Option<String>,
    pub operator: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl From<InventoryTransactionRequest> for InventoryTransactionQuery {
    fn from(value: InventoryTransactionRequest) -> Self {
        Self {
            transaction_id: value.transaction_id,
            material_id: value.material_id,
            movement_type: value.movement_type,
            batch_number: value.batch_number,
            from_bin: value.from_bin,
            to_bin: value.to_bin,
            reference_doc: value.reference_doc,
            operator: value.operator,
            date_from: value.date_from,
            date_to: value.date_to,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchRequest {
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub only_available: Option<bool>,
    pub only_expiring: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl From<BatchRequest> for BatchQuery {
    fn from(value: BatchRequest) -> Self {
        Self {
            material_id: value.material_id,
            batch_number: value.batch_number,
            quality_status: value.quality_status,
            only_available: value.only_available,
            only_expiring: value.only_expiring,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchHistoryRequest {
    pub event_type: Option<String>,
    pub operator: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl From<BatchHistoryRequest> for BatchHistoryQuery {
    fn from(value: BatchHistoryRequest) -> Self {
        Self {
            event_type: value.event_type,
            operator: value.operator,
            date_from: value.date_from,
            date_to: value.date_to,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapHistoryRequest {
    pub material_id: Option<String>,
    pub transaction_id: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl From<MapHistoryRequest> for MapHistoryQuery {
    fn from(value: MapHistoryRequest) -> Self {
        Self {
            material_id: value.material_id,
            transaction_id: value.transaction_id,
            date_from: value.date_from,
            date_to: value.date_to,
            page: value.page,
            page_size: value.page_size,
        }
    }
}
