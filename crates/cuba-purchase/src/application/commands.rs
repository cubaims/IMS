use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePurchaseOrderCommand {
    #[validate(length(min = 1))]
    pub supplier_id: String,

    pub expected_date: Option<NaiveDate>,

    pub remark: Option<String>,

    #[validate(length(min = 1))]
    pub lines: Vec<CreatePurchaseOrderLineCommand>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct CreatePurchaseOrderLineCommand {
    #[validate(range(min = 1))]
    pub line_no: i32,

    #[validate(length(min = 1))]
    pub material_id: String,

    #[validate(range(min = 1))]
    pub ordered_qty: i32,

    pub unit_price: Decimal,

    pub expected_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PostPurchaseReceiptCommand {
    #[serde(default)]
    pub po_id: String,

    pub posting_date: Option<DateTime<Utc>>,

    pub remark: Option<String>,

    #[validate(length(min = 1))]
    pub lines: Vec<PostPurchaseReceiptLineCommand>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct PostPurchaseReceiptLineCommand {
    #[validate(range(min = 1))]
    pub line_no: i32,

    #[validate(range(min = 1))]
    pub receipt_qty: i32,

    #[validate(length(min = 1))]
    pub batch_number: String,

    pub to_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PurchaseOrderQuery {
    pub supplier_id: Option<String>,
    pub material_id: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl PurchaseOrderQuery {
    pub fn limit(&self) -> i64 {
        self.page_size.unwrap_or(20).clamp(1, 200) as i64
    }

    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        ((page - 1) as i64) * self.limit()
    }
}
