use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSalesOrderRequest {
    pub customer_id: String,
    pub required_date: Option<Date>,
    pub remark: Option<String>,
    pub lines: Vec<CreateSalesOrderLineRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSalesOrderLineRequest {
    pub line_no: i32,
    pub material_id: String,
    pub ordered_qty: i32,
    pub unit_price: Decimal,
    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSalesOrderRequest {
    pub customer_id: Option<String>,
    pub required_date: Option<Date>,
    pub remark: Option<String>,
    pub lines: Option<Vec<CreateSalesOrderLineRequest>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostSalesShipmentRequest {
    pub posting_date: Option<OffsetDateTime>,
    pub pick_strategy: Option<String>,
    pub remark: Option<String>,
    pub lines: Vec<PostSalesShipmentLineRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostSalesShipmentLineRequest {
    pub line_no: i32,
    pub shipment_qty: i32,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewSalesFefoPickRequest {
    pub lines: Vec<PreviewSalesFefoPickLineRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewSalesFefoPickLineRequest {
    pub line_no: i32,
    pub shipment_qty: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesOrderCreatedResponse {
    pub so_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesShipmentResponse {
    pub so_id: String,
    pub status: String,
    pub transactions: Vec<SalesShipmentTransactionResponse>,
    pub reports_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesShipmentTransactionResponse {
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: i32,
    pub batch_number: Option<String>,
    pub from_bin: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FefoPickPreviewResponse {
    pub so_id: String,
    pub lines: Vec<FefoPickPreviewLineResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FefoPickPreviewLineResponse {
    pub line_no: i32,
    pub material_id: String,
    pub requested_qty: i32,
    pub picks: Vec<FefoPickPreviewBatchResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FefoPickPreviewBatchResponse {
    pub batch_number: String,
    pub bin_code: String,
    pub pick_qty: i32,
    pub expiry_date: Option<Date>,
    pub available_qty: i32,
}
