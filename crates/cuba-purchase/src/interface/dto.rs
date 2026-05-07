use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::application::{
    CreatePurchaseOrderCommand,
    CreatePurchaseOrderLineCommand,
    PostPurchaseReceiptCommand,
    PostPurchaseReceiptLineCommand,
    PurchaseOrderQuery,
};

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    pub supplier_id: String,
    pub expected_date: Option<Date>,
    pub remark: Option<String>,
    pub lines: Vec<CreatePurchaseOrderLineRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePurchaseOrderLineRequest {
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub unit_price: Decimal,
    pub expected_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostPurchaseReceiptRequest {
    pub posting_date: Option<OffsetDateTime>,
    pub remark: Option<String>,
    pub lines: Vec<PostPurchaseReceiptLineRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostPurchaseReceiptLineRequest {
    pub line_no: i32,
    pub receipt_qty: i32,
    pub batch_number: String,
    pub to_bin: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseOrderCreatedResponse {
    pub po_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReceiptResponse {
    pub po_id: String,
    pub status: String,
    pub transactions: Vec<PurchaseReceiptTransactionResponse>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReceiptTransactionResponse {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub to_bin: Option<String>,
}

// ====================== P0 修复：DTO → Command 转换 ======================

impl From<CreatePurchaseOrderRequest> for CreatePurchaseOrderCommand {
    fn from(req: CreatePurchaseOrderRequest) -> Self {
        Self {
            supplier_id: req.supplier_id,
            expected_date: req.expected_date,
            remark: req.remark,
            lines: req.lines.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CreatePurchaseOrderLineRequest> for CreatePurchaseOrderLineCommand {
    fn from(req: CreatePurchaseOrderLineRequest) -> Self {
        Self {
            line_no: req.line_no,
            material_id: req.material_id,
            ordered_qty: req.ordered_qty,
            unit_price: req.unit_price,
            expected_bin: req.expected_bin,
        }
    }
}

impl From<PostPurchaseReceiptRequest> for PostPurchaseReceiptCommand {
    fn from(req: PostPurchaseReceiptRequest) -> Self {
        Self {
            po_id: String::new(), // Path 参数后续在 handler 中注入
            posting_date: req.posting_date,
            remark: req.remark,
            lines: req.lines.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PostPurchaseReceiptLineRequest> for PostPurchaseReceiptLineCommand {
    fn from(req: PostPurchaseReceiptLineRequest) -> Self {
        Self {
            line_no: req.line_no,
            receipt_qty: req.receipt_qty,
            batch_number: req.batch_number,
            to_bin: req.to_bin,
        }
    }
}