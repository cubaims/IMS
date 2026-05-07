use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSalesOrderCommand {
    #[validate(length(min = 1))]
    pub customer_id: String,

    pub required_date: Option<Date>,

    pub remark: Option<String>,

    #[validate(length(min = 1))]
    pub lines: Vec<CreateSalesOrderLineCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateSalesOrderLineCommand {
    #[validate(range(min = 1))]
    pub line_no: i32,

    #[validate(length(min = 1))]
    pub material_id: String,

    #[validate(range(min = 1))]
    pub ordered_qty: i32,

    pub unit_price: Decimal,

    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PostSalesShipmentCommand {
    #[serde(default)]
    pub so_id: String,

    pub posting_date: Option<OffsetDateTime>,

    pub pick_strategy: Option<String>,

    pub remark: Option<String>,

    #[validate(length(min = 1))]
    pub lines: Vec<PostSalesShipmentLineCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PostSalesShipmentLineCommand {
    #[validate(range(min = 1))]
    pub line_no: i32,

    #[validate(range(min = 1))]
    pub shipment_qty: i32,

    pub batch_number: Option<String>,

    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PreviewSalesFefoPickCommand {
    #[serde(default)]
    pub so_id: String,

    pub lines: Vec<PreviewSalesFefoPickLineCommand>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PreviewSalesFefoPickLineCommand {
    #[validate(range(min = 1))]
    pub line_no: i32,

    #[validate(range(min = 1))]
    pub shipment_qty: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SalesOrderQuery {
    pub customer_id: Option<String>,
    pub material_id: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<Date>,
    pub date_to: Option<Date>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl SalesOrderQuery {
    pub fn limit(&self) -> i64 {
        self.page_size.unwrap_or(20).clamp(1, 200) as i64
    }

    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        ((page - 1) as i64) * self.limit()
    }
}
