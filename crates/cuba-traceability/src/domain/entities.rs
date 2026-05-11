use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};

use super::{BatchNumber, SerialNumber};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceabilitySummary {
    pub module: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceQueryOptions {
    pub max_depth: u32,
    pub movement_limit: u32,
    pub event_limit: u32,
    pub quality_limit: u32,
    pub include_genealogy: bool,
    pub include_inventory: bool,
    pub include_history: bool,
    pub include_quality: bool,
}

impl Default for TraceQueryOptions {
    fn default() -> Self {
        Self {
            max_depth: 10,
            movement_limit: 50,
            event_limit: 50,
            quality_limit: 50,
            include_genealogy: true,
            include_inventory: true,
            include_history: true,
            include_quality: true,
        }
    }
}

impl TraceQueryOptions {
    pub fn normalized(mut self) -> Self {
        self.max_depth = self.max_depth.clamp(1, 20);
        self.movement_limit = self.movement_limit.clamp(1, 200);
        self.event_limit = self.event_limit.clamp(1, 200);
        self.quality_limit = self.quality_limit.clamp(1, 200);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTraceQuery {
    pub batch_number: BatchNumber,
    pub options: TraceQueryOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialTraceQuery {
    pub serial_number: SerialNumber,
    pub include_batch_context: bool,
    pub options: TraceQueryOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSnapshot {
    pub batch_number: String,
    pub material_id: String,
    pub material_name: String,
    pub production_date: Option<Date>,
    pub expiry_date: Option<Date>,
    pub supplier_batch: Option<String>,
    pub quality_grade: Option<String>,
    pub current_stock: Decimal,
    pub current_bin: Option<String>,
    pub quality_status: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialSnapshot {
    pub serial_number: String,
    pub material_id: String,
    pub material_name: String,
    pub batch_number: Option<String>,
    pub current_status: String,
    pub current_bin: Option<String>,
    pub quality_status: String,
    pub last_movement_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenealogyTrace {
    pub backward_components: Vec<BatchGenealogyLink>,
    pub forward_where_used: Vec<BatchGenealogyLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenealogyLink {
    pub level: i32,
    pub parent_batch_number: String,
    pub component_batch_number: String,
    pub parent_material_id: String,
    pub component_material_id: String,
    pub production_order_id: Option<String>,
    pub consumed_qty: Decimal,
    pub output_qty: Option<Decimal>,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryMovementTrace {
    pub transaction_id: String,
    pub transaction_date: OffsetDateTime,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: Decimal,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub reference_doc: Option<String>,
    pub operator: Option<String>,
    pub quality_status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHistoryTrace {
    pub history_id: i64,
    pub batch_number: String,
    pub material_id: String,
    pub event_type: String,
    pub old_quality_status: Option<String>,
    pub new_quality_status: Option<String>,
    pub old_bin: Option<String>,
    pub new_bin: Option<String>,
    pub old_stock: Option<Decimal>,
    pub new_stock: Option<Decimal>,
    pub qty_change: Option<Decimal>,
    pub transaction_id: Option<String>,
    pub inspection_lot_id: Option<String>,
    pub notification_id: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: OffsetDateTime,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialHistoryTrace {
    pub history_id: i64,
    pub serial_number: String,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub old_bin: Option<String>,
    pub new_bin: Option<String>,
    pub old_quality_status: Option<String>,
    pub new_quality_status: Option<String>,
    pub transaction_id: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionLotTrace {
    pub inspection_lot_id: String,
    pub material_id: String,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub inspection_type: String,
    pub lot_status: String,
    pub inspection_date: Option<OffsetDateTime>,
    pub inspector: Option<String>,
    pub inspection_result: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityNotificationTrace {
    pub notification_id: String,
    pub inspection_lot_id: Option<String>,
    pub material_id: String,
    pub batch_number: Option<String>,
    pub serial_number: Option<String>,
    pub defect_code: Option<String>,
    pub problem_description: String,
    pub severity: Option<String>,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub responsible_person: Option<String>,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub closed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTraceReport {
    pub batch: BatchSnapshot,
    pub genealogy: Option<BatchGenealogyTrace>,
    pub inventory_movements: Vec<InventoryMovementTrace>,
    pub batch_history: Vec<BatchHistoryTrace>,
    pub inspection_lots: Vec<InspectionLotTrace>,
    pub quality_notifications: Vec<QualityNotificationTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialTraceReport {
    pub serial: SerialSnapshot,
    pub serial_history: Vec<SerialHistoryTrace>,
    pub inventory_movements: Vec<InventoryMovementTrace>,
    pub inspection_lots: Vec<InspectionLotTrace>,
    pub quality_notifications: Vec<QualityNotificationTrace>,
    pub batch_context: Option<Box<BatchTraceReport>>,
}
