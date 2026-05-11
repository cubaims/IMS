use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::application::{
    BatchHistoryQuery, BatchQuery, CurrentStockQuery, InventoryTransactionQuery,
    ListInventoryCountsInput, MapHistoryQuery, PageQuery, PickBatchFefoCommand,
    PostInventoryCommand, TransferInventoryCommand,
};

use crate::domain::{
    InventoryCount, InventoryCountLine, InventoryCountLineStatus, InventoryCountMovementType,
    InventoryCountScope, InventoryCountStatus, InventoryCountType,
};

// ====================== 盘点模块 DTO ======================
/// 创建盘点单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateInventoryCountRequest {
    pub count_type: InventoryCountType,
    pub count_scope: InventoryCountScope,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub remark: Option<String>,
}

/// 创建盘点单响应
#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryCountResponse {
    pub count_doc_id: String,
    pub status: InventoryCountStatus,
}

/// 盘点单详情响应
#[derive(Debug, Clone, Serialize)]
pub struct InventoryCountResponse {
    pub count_doc_id: String,
    pub count_type: InventoryCountType,
    pub count_scope: InventoryCountScope,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub status: InventoryCountStatus,

    pub created_by: String,
    pub approved_by: Option<String>,
    pub posted_by: Option<String>,

    pub created_at: OffsetDateTime,
    pub approved_at: Option<OffsetDateTime>,
    pub posted_at: Option<OffsetDateTime>,
    pub closed_at: Option<OffsetDateTime>,

    pub remark: Option<String>,
    pub lines: Vec<InventoryCountLineResponse>,
}

/// 查询盘点单列表请求
#[derive(Debug, Clone, Deserialize)]
pub struct ListInventoryCountsRequest {
    pub status: Option<InventoryCountStatus>,
    pub count_type: Option<InventoryCountType>,
    pub count_scope: Option<InventoryCountScope>,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub created_by: Option<String>,

    #[serde(default, with = "time::serde::rfc3339::option")]
    pub date_from: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub date_to: Option<OffsetDateTime>,

    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl From<ListInventoryCountsRequest> for ListInventoryCountsInput {
    fn from(value: ListInventoryCountsRequest) -> Self {
        Self {
            status: value.status,
            count_type: value.count_type,
            count_scope: value.count_scope,
            zone_code: value.zone_code,
            bin_code: value.bin_code,
            material_id: value.material_id,
            batch_number: value.batch_number,
            created_by: value.created_by,
            date_from: value.date_from,
            date_to: value.date_to,
            page: PageQuery {
                page: value.page.map(u64::from),
                page_size: value.page_size.map(u64::from),
                sort_by: value.sort_by,
                sort_order: value.sort_order,
            },
        }
    }
}

impl From<InventoryCount> for InventoryCountResponse {
    fn from(value: InventoryCount) -> Self {
        Self {
            count_doc_id: value.count_doc_id,
            count_type: value.count_type,
            count_scope: value.count_scope,
            zone_code: value.zone_code,
            bin_code: value.bin_code,
            material_id: value.material_id,
            batch_number: value.batch_number,
            status: value.status,
            created_by: value.created_by,
            approved_by: value.approved_by,
            posted_by: value.posted_by,
            created_at: value.created_at,
            approved_at: value.approved_at,
            posted_at: value.posted_at,
            closed_at: value.closed_at,
            remark: value.remark,
            lines: value
                .lines
                .into_iter()
                .map(InventoryCountLineResponse::from)
                .collect(),
        }
    }
}

/// 盘点明细响应
#[derive(Debug, Clone, Serialize)]
pub struct InventoryCountLineResponse {
    pub count_doc_id: String,
    pub line_no: i32,

    pub material_id: String,
    pub bin_code: String,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,

    pub system_qty: Decimal,
    pub counted_qty: Option<Decimal>,
    pub difference_qty: Option<Decimal>,
    pub difference_reason: Option<String>,

    pub movement_type: Option<String>,

    pub transaction_id: Option<String>,
    pub status: InventoryCountLineStatus,
    pub remark: Option<String>,
}

impl From<InventoryCountLine> for InventoryCountLineResponse {
    fn from(value: InventoryCountLine) -> Self {
        Self {
            count_doc_id: value.count_doc_id,
            line_no: value.line_no,
            material_id: value.material_id,
            bin_code: value.bin_code,
            batch_number: value.batch_number,
            quality_status: value.quality_status,
            system_qty: value.system_qty,
            counted_qty: value.counted_qty,
            difference_qty: value.difference_qty,
            difference_reason: value.difference_reason,
            movement_type: value
                .movement_type
                .as_ref()
                .map(InventoryCountMovementType::as_code)
                .map(str::to_string),
            transaction_id: value.transaction_id,
            status: value.status,
            remark: value.remark,
        }
    }
}

/// 录入单行实盘数量请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInventoryCountLineRequest {
    pub counted_qty: Decimal,
    pub difference_reason: Option<String>,
    pub remark: Option<String>,
}

/// 批量录入实盘数量请求
#[derive(Debug, Clone, Deserialize)]
pub struct BatchUpdateInventoryCountLinesRequest {
    pub lines: Vec<BatchUpdateInventoryCountLineItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchUpdateInventoryCountLineItem {
    pub line_no: i32,
    pub counted_qty: Decimal,
    pub difference_reason: Option<String>,
    pub remark: Option<String>,
}

/// 提交盘点单请求
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitInventoryCountRequest {
    pub remark: Option<String>,
}

/// 审核盘点单请求
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveInventoryCountRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

/// 盘点过账请求
#[derive(Debug, Clone, Deserialize)]
pub struct PostInventoryCountRequest {
    #[serde(with = "time::serde::rfc3339")]
    pub posting_date: OffsetDateTime,
    pub remark: Option<String>,
}

/// 关闭盘点单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CloseInventoryCountRequest {
    pub remark: Option<String>,
}

/// 取消盘点单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CancelInventoryCountRequest {
    pub remark: Option<String>,
}

// ====================== 库存核心 DTO（保持不变） ======================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInventoryRequest {
    pub material_id: String,
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
    pub quantity: Decimal,
    pub from_bin: String,
    pub to_bin: String,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub quality_status: Option<String>,
    pub remark: Option<String>,
    pub posting_date: Option<OffsetDateTime>,
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
    pub quantity: Decimal,
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
    pub only_low_stock: Option<bool>,
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
            only_low_stock: value.only_low_stock,
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
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
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
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
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
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
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

impl From<BatchUpdateInventoryCountLineItem>
    for crate::application::inventory_count_model::BatchUpdateInventoryCountLineItem
{
    fn from(dto: BatchUpdateInventoryCountLineItem) -> Self {
        Self {
            line_no: dto.line_no,
            counted_qty: dto.counted_qty,
            difference_reason: dto.difference_reason,
            remark: dto.remark,
        }
    }
}
