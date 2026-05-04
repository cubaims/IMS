use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    pub supplier_id: String,
    pub expected_date: Option<NaiveDate>,
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
    pub posting_date: Option<DateTime<Utc>>,
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
