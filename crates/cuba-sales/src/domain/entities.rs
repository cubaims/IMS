use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrder {
    pub so_id: String,
    pub customer_id: String,
    pub so_date: NaiveDate,
    pub required_date: Option<NaiveDate>,
    pub status: SalesOrderStatus,
    pub remark: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<SalesOrderLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderLine {
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub shipped_qty: i32,
    pub unit_price: Decimal,
    pub from_bin: Option<String>,
    pub line_status: SalesLineStatus,
}

impl SalesOrderLine {
    pub fn open_qty(&self) -> i32 {
        self.ordered_qty - self.shipped_qty
    }

    pub fn can_ship(&self, shipment_qty: i32) -> bool {
        shipment_qty > 0 && shipment_qty <= self.open_qty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesOrderStatus {
    Draft,
    Open,
    PartiallyShipped,
    Shipped,
    Closed,
    Cancelled,
}

impl SalesOrderStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Open => "已下达",
            Self::PartiallyShipped => "部分发货",
            Self::Shipped => "完成",
            Self::Closed => "关闭",
            Self::Cancelled => "取消",
        }
    }

    pub fn can_ship(self) -> bool {
        matches!(self, Self::Open | Self::PartiallyShipped)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesLineStatus {
    Open,
    PartiallyShipped,
    Completed,
    Cancelled,
}

impl SalesLineStatus {
    pub fn as_db_text(self) -> &'static str {
        match self {
            Self::Open => "打开",
            Self::PartiallyShipped => "部分发货",
            Self::Completed => "完成",
            Self::Cancelled => "取消",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PickStrategy {
    Fefo,
    Manual,
}

impl PickStrategy {
    pub fn is_fefo(&self) -> bool {
        matches!(self, Self::Fefo)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesShipmentResult {
    pub so_id: String,
    pub status: String,
    pub transactions: Vec<SalesShipmentTransaction>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesShipmentTransaction {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FefoPickLine {
    pub material_id: String,
    pub requested_qty: i32,
    pub picks: Vec<FefoPickBatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FefoPickBatch {
    pub batch_number: String,
    pub bin_code: String,
    pub pick_qty: i32,
    pub expiry_date: Option<NaiveDate>,
    pub available_qty: i32,
}
