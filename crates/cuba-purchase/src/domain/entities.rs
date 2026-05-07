use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub po_id: String,
    pub supplier_id: String,
    pub po_date: Date,
    pub expected_date: Option<Date>,
    pub status: PurchaseOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub lines: Vec<PurchaseOrderLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub received_qty: i32,
    pub unit_price: Decimal,
    pub expected_bin: Option<String>,
    pub line_status: PurchaseLineStatus,
}

impl PurchaseOrderLine {
    pub fn open_qty(&self) -> i32 {
        self.ordered_qty - self.received_qty
    }

    pub fn can_receive(&self, receipt_qty: i32) -> bool {
        receipt_qty > 0 && receipt_qty <= self.open_qty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseOrderStatus {
    Draft,
    Open,
    PartiallyReceived,
    Received,
    Closed,
    Cancelled,
}

impl PurchaseOrderStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Open => "已下达",
            Self::PartiallyReceived => "部分到货",
            Self::Received => "完成",
            Self::Closed => "关闭",
            Self::Cancelled => "取消",
        }
    }

    pub fn can_receive(self) -> bool {
        matches!(self, Self::Open | Self::PartiallyReceived)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseLineStatus {
    Open,
    PartiallyReceived,
    Completed,
    Cancelled,
}

impl PurchaseLineStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Open => "打开",
            Self::PartiallyReceived => "部分到货",
            Self::Completed => "完成",
            Self::Cancelled => "取消",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptResult {
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
