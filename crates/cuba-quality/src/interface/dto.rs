use crate::application::{
    AddInspectionResultCommand, CreateInspectionLotCommand, FreezeBatchCommand,
    MakeInspectionDecisionCommand, ScrapBatchCommand, UnfreezeBatchCommand,
};
use crate::domain::{
    BatchNumber, BatchQualityStatus, DefectCode, InspectionCharId,
    InspectionDecision, InspectionLotId, InspectionLotType,
    InspectionResultStatus, MaterialId, Operator,
    QualityNotificationSeverity,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 创建检验批请求。
///
/// 对应：
/// POST /api/quality/inspection-lots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionLotRequest {
    pub lot_type: InspectionLotType,
    pub material_id: String,
    pub batch_number: String,
    pub source_transaction_id: Option<String>,
    pub source_doc: Option<String>,
    pub quantity: Decimal,
    pub sample_qty: Decimal,
    pub remark: Option<String>,

    /// 是否创建后把批次标记为“待检”。
    ///
    /// 采购入库 / 生产入库自动生成检验批时建议为 true。
    pub mark_batch_pending_inspection: Option<bool>,
}

impl CreateInspectionLotRequest {
    pub fn into_command(self, operator: Operator) -> CreateInspectionLotCommand {
        CreateInspectionLotCommand {
            lot_type: self.lot_type,
            material_id: MaterialId::new(self.material_id),
            batch_number: BatchNumber::new(self.batch_number),
            source_transaction_id: self.source_transaction_id,
            source_doc: self.source_doc,
            quantity: self.quantity,
            sample_qty: self.sample_qty,
            created_by: operator,
            remark: self.remark,
            mark_batch_pending_inspection: self
                .mark_batch_pending_inspection
                .unwrap_or(true),
        }
    }
}

/// 创建检验批响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionLotResponse {
    pub inspection_lot_id: String,
    pub batch_number: String,
    pub batch_status_changed: bool,
}

/// 录入单条检验结果请求。
///
/// 对应：
/// POST /api/quality/inspection-lots/{lot_id}/results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddInspectionResultRequest {
    pub char_id: String,
    pub measured_value: Option<Decimal>,
    pub qualitative_result: Option<InspectionResultStatus>,
    pub defect_code: Option<String>,
    pub defect_qty: Option<Decimal>,
    pub remark: Option<String>,
}

impl AddInspectionResultRequest {
    pub fn into_command(
        self,
        lot_id: String,
        operator: Operator,
    ) -> AddInspectionResultCommand {
        AddInspectionResultCommand {
            inspection_lot_id: InspectionLotId::new(lot_id),
            char_id: InspectionCharId::new(self.char_id),
            measured_value: self.measured_value,
            qualitative_result: self.qualitative_result,
            defect_code: self.defect_code.map(DefectCode::new),
            defect_qty: self.defect_qty.unwrap_or(Decimal::ZERO),
            inspector: operator,
            remark: self.remark,
        }
    }
}

/// 录入检验结果响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddInspectionResultResponse {
    pub result_id: String,
    pub result_status: InspectionResultStatus,
}

/// 批量录入检验结果请求。
///
/// 对应：
/// POST /api/quality/inspection-lots/{lot_id}/results/batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAddInspectionResultsRequest {
    pub results: Vec<AddInspectionResultRequest>,
}

/// 批量录入检验结果响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAddInspectionResultsResponse {
    pub results: Vec<AddInspectionResultResponse>,
}

/// 质量判定请求。
///
/// 对应：
/// POST /api/quality/inspection-lots/{lot_id}/decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeInspectionDecisionRequest {
    pub decision: InspectionDecision,
    pub reason: String,
    pub defect_code: Option<String>,
    pub create_notification: Option<bool>,
    pub notification_severity: Option<QualityNotificationSeverity>,
    pub remark: Option<String>,
}

impl MakeInspectionDecisionRequest {
    pub fn into_command(
        self,
        lot_id: String,
        operator: Operator,
    ) -> MakeInspectionDecisionCommand {
        MakeInspectionDecisionCommand {
            inspection_lot_id: InspectionLotId::new(lot_id),
            decision: self.decision,
            reason: self.reason,
            defect_code: self.defect_code.map(DefectCode::new),
            create_notification: self.create_notification.unwrap_or(false),
            notification_severity: self.notification_severity,
            decided_by: operator,
            remark: self.remark,
        }
    }
}

/// 质量判定响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeInspectionDecisionResponse {
    pub inspection_lot_id: String,
    pub decision: InspectionDecision,
    pub notification_id: Option<String>,
}

/// 冻结批次请求。
///
/// 对应：
/// POST /api/quality/batches/{batch_number}/freeze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeBatchRequest {
    pub reason: String,
    pub reference_doc: Option<String>,
    pub remark: Option<String>,
}

impl FreezeBatchRequest {
    pub fn into_command(
        self,
        batch_number: String,
        operator: Operator,
    ) -> FreezeBatchCommand {
        FreezeBatchCommand {
            batch_number: BatchNumber::new(batch_number),
            reason: self.reason,
            reference_doc: self.reference_doc,
            operator,
            remark: self.remark,
        }
    }
}

/// 解冻批次请求。
///
/// 对应：
/// POST /api/quality/batches/{batch_number}/unfreeze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfreezeBatchRequest {
    pub target_status: BatchQualityStatus,
    pub reason: String,
    pub reference_doc: Option<String>,
    pub remark: Option<String>,
}

impl UnfreezeBatchRequest {
    pub fn into_command(
        self,
        batch_number: String,
        operator: Operator,
    ) -> UnfreezeBatchCommand {
        UnfreezeBatchCommand {
            batch_number: BatchNumber::new(batch_number),
            target_status: self.target_status,
            reason: self.reason,
            reference_doc: self.reference_doc,
            operator,
            remark: self.remark,
        }
    }
}

/// 报废批次请求。
///
/// 注意：
/// 这里是质量报废，不自动扣库存。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapBatchRequest {
    pub reason: String,
    pub defect_code: Option<String>,
    pub reference_doc: Option<String>,
    pub remark: Option<String>,
}

impl ScrapBatchRequest {
    pub fn into_command(
        self,
        batch_number: String,
        operator: Operator,
    ) -> ScrapBatchCommand {
        ScrapBatchCommand {
            batch_number: BatchNumber::new(batch_number),
            reason: self.reason,
            defect_code: self.defect_code.map(DefectCode::new),
            reference_doc: self.reference_doc,
            operator,
            remark: self.remark,
        }
    }
}

/// 批次状态操作响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchActionResponse {
    pub batch_number: String,
    pub success: bool,
}

/// 当前用户上下文。
///
/// MVP 先用简单结构。
/// 后续可以从 cuba-auth 的 JWT middleware 注入。
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub username: String,
}

impl CurrentUser {
    pub fn operator(&self) -> Operator {
        Operator::new(self.username.clone())
    }
}