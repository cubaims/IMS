use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderCreated {
    pub po_id: String,
    pub status: String,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderSummary {
    pub po_id: String,
    pub supplier_id: String,
    pub supplier_name: String,
    pub po_date: Date,
    pub expected_date: Option<Date>,
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub status: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderDetail {
    pub po_id: String,
    pub supplier_id: String,
    pub po_date: Date,
    pub expected_date: Option<Date>,
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub status: Option<String>,
    pub created_by: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<OffsetDateTime>,
    pub notes: Option<String>,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
    pub lines: Vec<PurchaseOrderLineDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLineDetail {
    pub id: i64,
    pub po_id: String,
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub received_qty: Option<i32>,
    pub open_qty: Option<i32>,
    pub unit_price: Decimal,
    pub line_amount: Option<Decimal>,
    pub expected_bin: Option<String>,
    pub line_status: Option<String>,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptPosted {
    pub po_id: String,
    pub status: String,
    pub transactions: Vec<PurchaseReceiptTransaction>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptTransaction {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub to_bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderClosed {
    pub po_id: String,
    pub status: String,
}
