use crate::domain::{
    BatchNumber, DefectCode, MaterialId, Operator, QualityError, QualityNotificationId,
    QualityNotificationSeverity, QualityNotificationStatus, QualityResult,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// 质量通知。
///
/// 用于记录质量异常、责任人、原因分析和纠正措施。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityNotification {
    pub id: QualityNotificationId,

    /// 来源类型，例如 INSPECTION_LOT。
    pub source_type: String,

    /// 来源 ID，例如检验批 ID。
    pub source_id: String,

    pub material_id: MaterialId,
    pub batch_number: BatchNumber,

    pub defect_code: Option<DefectCode>,
    pub defect_qty: Decimal,

    pub severity: QualityNotificationSeverity,
    pub status: QualityNotificationStatus,

    pub description: String,

    /// 负责人。
    pub owner: Option<Operator>,

    /// 原因分析。
    pub root_cause: Option<String>,

    /// 纠正措施。
    pub corrective_action: Option<String>,

    pub created_by: Operator,
    pub created_at: OffsetDateTime,

    pub closed_by: Option<Operator>,
    pub closed_at: Option<OffsetDateTime>,

    pub remark: Option<String>,
}

impl QualityNotification {
    /// 创建质量通知。
    pub fn create(input: CreateQualityNotification) -> QualityResult<Self> {
        if input.description.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("description"));
        }

        if input.defect_qty < Decimal::ZERO {
            return Err(QualityError::BusinessRuleViolation(
                "不良数量不能小于 0".to_string(),
            ));
        }

        Ok(Self {
            id: input.id,
            source_type: input.source_type,
            source_id: input.source_id,
            material_id: input.material_id,
            batch_number: input.batch_number,
            defect_code: input.defect_code,
            defect_qty: input.defect_qty,
            severity: input.severity,
            status: QualityNotificationStatus::Open,
            description: input.description,
            owner: input.owner,
            root_cause: None,
            corrective_action: None,
            created_by: input.created_by,
            created_at: input.now,
            closed_by: None,
            closed_at: None,
            remark: input.remark,
        })
    }

    /// 分配负责人。
    pub fn assign(&mut self, owner: Operator) -> QualityResult<()> {
        if !self.status.can_update() {
            return Err(QualityError::QualityNotificationStatusInvalid);
        }

        self.owner = Some(owner);
        self.status = QualityNotificationStatus::InProgress;

        Ok(())
    }

    /// 解决质量通知。
    pub fn resolve(
        &mut self,
        root_cause: String,
        corrective_action: String,
    ) -> QualityResult<()> {
        if !self.status.can_update() {
            return Err(QualityError::QualityNotificationStatusInvalid);
        }

        if root_cause.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("root_cause"));
        }

        if corrective_action.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("corrective_action"));
        }

        self.root_cause = Some(root_cause);
        self.corrective_action = Some(corrective_action);
        self.status = QualityNotificationStatus::Resolved;

        Ok(())
    }

    /// 关闭质量通知。
    pub fn close(&mut self, operator: Operator, now: OffsetDateTime) -> QualityResult<()> {
        if !self.status.can_close() {
            return Err(QualityError::QualityNotificationStatusInvalid);
        }

        self.status = QualityNotificationStatus::Closed;
        self.closed_by = Some(operator);
        self.closed_at = Some(now);

        Ok(())
    }

    /// 取消质量通知。
    pub fn cancel(&mut self) -> QualityResult<()> {
        if !self.status.can_update() {
            return Err(QualityError::QualityNotificationStatusInvalid);
        }

        self.status = QualityNotificationStatus::Cancelled;

        Ok(())
    }
}

/// 创建质量通知输入。
#[derive(Debug, Clone)]
pub struct CreateQualityNotification {
    pub id: QualityNotificationId,
    pub source_type: String,
    pub source_id: String,
    pub material_id: MaterialId,
    pub batch_number: BatchNumber,
    pub defect_code: Option<DefectCode>,
    pub defect_qty: Decimal,
    pub severity: QualityNotificationSeverity,
    pub description: String,
    pub owner: Option<Operator>,
    pub created_by: Operator,
    pub now: OffsetDateTime,
    pub remark: Option<String>,
}