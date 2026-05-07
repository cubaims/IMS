use cuba_shared::AppError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::{
    BatchNumber, BatchQualityStatus, DefectCode, InspectionCharId, InspectionDecision,
    InspectionLotId, InspectionLotType, InspectionResultId, InspectionResultStatus, MaterialId,
    Operator, QualityError, QualityNotificationId, QualityNotificationSeverity,
};

#[derive(Debug, thiserror::Error)]
pub enum QualityApplicationError {
    #[error("质量模块尚未实现: {0}")]
    NotImplemented(&'static str),

    #[error("领域规则错误: {0}")]
    Domain(#[from] QualityError),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("序列化错误: {0}")]
    Serialization(String),
}

impl From<QualityApplicationError> for AppError {
    fn from(err: QualityApplicationError) -> Self {
        match err {
            QualityApplicationError::Domain(domain_err) => {
                AppError::Validation(domain_err.to_string())
            }
            QualityApplicationError::NotImplemented(message) => {
                AppError::Internal(message.to_string())
            }
            QualityApplicationError::Database(message) => {
                AppError::Internal(format!("质量模块数据库错误: {message}"))
            }
            QualityApplicationError::Serialization(message) => {
                AppError::Internal(format!("质量模块序列化错误: {message}"))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InspectionLotQuery {
    pub material_id: Option<String>,
    pub batch_number: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchHistoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CreateInspectionLotCommand {
    pub lot_type: InspectionLotType,
    pub material_id: MaterialId,
    pub batch_number: BatchNumber,
    pub source_transaction_id: Option<String>,
    pub source_doc: Option<String>,
    pub quantity: Decimal,
    pub sample_qty: Decimal,
    pub created_by: Operator,
    pub remark: Option<String>,
    pub mark_batch_pending_inspection: bool,
}

#[derive(Debug, Clone)]
pub struct AddInspectionResultCommand {
    pub inspection_lot_id: InspectionLotId,
    pub char_id: InspectionCharId,
    pub measured_value: Option<Decimal>,
    pub qualitative_result: Option<InspectionResultStatus>,
    pub defect_code: Option<DefectCode>,
    pub defect_qty: Decimal,
    pub inspector: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MakeInspectionDecisionCommand {
    pub inspection_lot_id: InspectionLotId,
    pub decision: InspectionDecision,
    pub reason: String,
    pub defect_code: Option<DefectCode>,
    pub create_notification: bool,
    pub notification_severity: Option<QualityNotificationSeverity>,
    pub decided_by: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FreezeBatchCommand {
    pub batch_number: BatchNumber,
    pub reason: String,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnfreezeBatchCommand {
    pub batch_number: BatchNumber,
    pub target_status: BatchQualityStatus,
    pub reason: String,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScrapBatchCommand {
    pub batch_number: BatchNumber,
    pub reason: String,
    pub defect_code: Option<DefectCode>,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateInspectionLotOutput {
    pub inspection_lot_id: InspectionLotId,
    pub batch_number: BatchNumber,
    pub batch_status_changed: bool,
}

#[derive(Debug, Clone)]
pub struct AddInspectionResultOutput {
    pub result_id: InspectionResultId,
    pub result_status: InspectionResultStatus,
}

#[derive(Debug, Clone)]
pub struct MakeInspectionDecisionOutput {
    pub inspection_lot_id: InspectionLotId,
    pub decision: InspectionDecision,
    pub notification_id: Option<QualityNotificationId>,
}

#[derive(Debug, Clone)]
pub struct BatchActionOutput {
    pub batch_number: BatchNumber,
}

#[derive(Debug, Clone, Default)]
pub struct CreateInspectionLotUseCase;

impl CreateInspectionLotUseCase {
    pub fn new<A, B, C, G>(_a: A, _b: B, _c: C, _g: G) -> Self {
        Self
    }

    pub async fn execute(
        &self,
        command: CreateInspectionLotCommand,
    ) -> Result<CreateInspectionLotOutput, QualityApplicationError> {
        Ok(CreateInspectionLotOutput {
            inspection_lot_id: InspectionLotId::new("IL-MVP-PLACEHOLDER"),
            batch_number: command.batch_number,
            batch_status_changed: command.mark_batch_pending_inspection,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct AddInspectionResultUseCase;

impl AddInspectionResultUseCase {
    pub fn new<A, B, C, G>(_a: A, _b: B, _c: C, _g: G) -> Self {
        Self
    }

    pub async fn execute(
        &self,
        command: AddInspectionResultCommand,
    ) -> Result<AddInspectionResultOutput, QualityApplicationError> {
        Ok(AddInspectionResultOutput {
            result_id: InspectionResultId::new("IR-MVP-PLACEHOLDER"),
            result_status: command.qualitative_result.unwrap_or(InspectionResultStatus::Pass),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct MakeInspectionDecisionUseCase;

impl MakeInspectionDecisionUseCase {
    pub fn new<A, B, C, D, E, G>(_a: A, _b: B, _c: C, _d: D, _e: E, _g: G) -> Self {
        Self
    }

    pub async fn execute(
        &self,
        command: MakeInspectionDecisionCommand,
    ) -> Result<MakeInspectionDecisionOutput, QualityApplicationError> {
        Ok(MakeInspectionDecisionOutput {
            inspection_lot_id: command.inspection_lot_id,
            decision: command.decision,
            notification_id: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct FreezeBatchUseCase;

impl FreezeBatchUseCase {
    pub fn new<S>(_store: S) -> Self {
        Self
    }

    pub async fn execute(
        &self,
        command: FreezeBatchCommand,
    ) -> Result<BatchActionOutput, QualityApplicationError> {
        Ok(BatchActionOutput {
            batch_number: command.batch_number,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnfreezeBatchUseCase;

impl UnfreezeBatchUseCase {
    pub fn new<S>(_store: S) -> Self {
        Self
    }

    pub async fn execute(
        &self,
        command: UnfreezeBatchCommand,
    ) -> Result<BatchActionOutput, QualityApplicationError> {
        Ok(BatchActionOutput {
            batch_number: command.batch_number,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScrapBatchUseCase;

impl ScrapBatchUseCase {
    pub fn new<S>(_store: S) -> Self {
        Self
    }

    pub async fn execute(
        &self,
        command: ScrapBatchCommand,
    ) -> Result<BatchActionOutput, QualityApplicationError> {
        Ok(BatchActionOutput {
            batch_number: command.batch_number,
        })
    }
}
