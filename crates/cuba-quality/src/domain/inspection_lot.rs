use crate::domain::{
    BatchNumber, InspectionDecision, InspectionLotId, InspectionLotStatus, InspectionLotType,
    MaterialId, Operator, QualityError, QualityResult,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// 检验批聚合。
///
/// 检验批是质量管理的核心对象。
/// 一般来源于采购入库、生产入库或手工创建。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionLot {
    pub id: InspectionLotId,

    /// 检验批类型。
    pub lot_type: InspectionLotType,

    /// 物料 ID。
    pub material_id: MaterialId,

    /// 批次号。
    pub batch_number: BatchNumber,

    /// 来源库存事务 ID。
    pub source_transaction_id: Option<String>,

    /// 来源单据，例如 PO、生产订单等。
    pub source_doc: Option<String>,

    /// 检验数量。
    pub quantity: Decimal,

    /// 抽样数量。
    pub sample_qty: Decimal,

    /// 检验批状态。
    pub status: InspectionLotStatus,

    /// 最终质量判定。
    pub decision: Option<InspectionDecision>,

    /// 质量判定原因。
    pub decision_reason: Option<String>,

    /// 创建人。
    pub created_by: Operator,

    /// 检验人。
    pub inspected_by: Option<Operator>,

    /// 判定人。
    pub decided_by: Option<Operator>,

    pub created_at: OffsetDateTime,
    pub inspected_at: Option<OffsetDateTime>,
    pub decided_at: Option<OffsetDateTime>,
    pub closed_at: Option<OffsetDateTime>,

    /// 备注。
    pub remark: Option<String>,
}

impl InspectionLot {
    /// 创建新的检验批。
    ///
    /// 这里只做领域规则校验，不访问数据库。
    pub fn create(input: CreateInspectionLot) -> QualityResult<Self> {
        if input.quantity <= Decimal::ZERO {
            return Err(QualityError::QuantityMustBePositive);
        }

        if input.sample_qty < Decimal::ZERO {
            return Err(QualityError::BusinessRuleViolation(
                "样本数量不能小于 0".to_string(),
            ));
        }

        if input.sample_qty > input.quantity {
            return Err(QualityError::SampleQtyExceeded);
        }

        Ok(Self {
            id: input.id,
            lot_type: input.lot_type,
            material_id: input.material_id,
            batch_number: input.batch_number,
            source_transaction_id: input.source_transaction_id,
            source_doc: input.source_doc,
            quantity: input.quantity,
            sample_qty: input.sample_qty,
            status: InspectionLotStatus::Created,
            decision: None,
            decision_reason: None,
            created_by: input.created_by,
            inspected_by: None,
            decided_by: None,
            created_at: input.now,
            inspected_at: None,
            decided_at: None,
            closed_at: None,
            remark: input.remark,
        })
    }

    /// 标记为检验中。
    pub fn mark_in_progress(
        &mut self,
        operator: Operator,
        now: OffsetDateTime,
    ) -> QualityResult<()> {
        if !self.status.can_enter_result() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        self.status = InspectionLotStatus::InProgress;
        self.inspected_by = Some(operator);
        self.inspected_at = Some(now);

        Ok(())
    }

    /// 提交检验结果。
    pub fn submit_results(&mut self, operator: Operator, now: OffsetDateTime) -> QualityResult<()> {
        if !self.status.can_submit_result() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        self.status = InspectionLotStatus::ResultEntered;
        self.inspected_by = Some(operator);
        self.inspected_at = Some(now);

        Ok(())
    }

    /// 更新检验批基础信息。
    pub fn update_details(&mut self, input: UpdateInspectionLotDetails) -> QualityResult<()> {
        if !matches!(
            self.status,
            InspectionLotStatus::Created | InspectionLotStatus::InProgress
        ) {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        if input.quantity <= Decimal::ZERO {
            return Err(QualityError::QuantityMustBePositive);
        }

        if input.sample_qty < Decimal::ZERO {
            return Err(QualityError::BusinessRuleViolation(
                "样本数量不能小于 0".to_string(),
            ));
        }

        if input.sample_qty > input.quantity {
            return Err(QualityError::SampleQtyExceeded);
        }

        self.source_transaction_id = input.source_transaction_id;
        self.source_doc = input.source_doc;
        self.quantity = input.quantity;
        self.sample_qty = input.sample_qty;
        self.remark = input.remark;

        Ok(())
    }

    /// 做质量判定。
    ///
    /// 兼容旧调用点。新代码建议使用 make_decision_with_reason。
    pub fn make_decision(
        &mut self,
        decision: InspectionDecision,
        operator: Operator,
        now: OffsetDateTime,
    ) -> QualityResult<()> {
        self.apply_decision(decision, None, operator, now)
    }

    /// 做带原因的质量判定。
    ///
    /// 质量判定原因必须填写，方便后续写入批次历史和审计日志。
    pub fn make_decision_with_reason(
        &mut self,
        decision: InspectionDecision,
        reason: String,
        operator: Operator,
        now: OffsetDateTime,
    ) -> QualityResult<()> {
        let reason = reason.trim().to_string();

        if reason.is_empty() {
            return Err(QualityError::QualityDecisionReasonRequired);
        }

        self.apply_decision(decision, Some(reason), operator, now)
    }

    fn apply_decision(
        &mut self,
        decision: InspectionDecision,
        reason: Option<String>,
        operator: Operator,
        now: OffsetDateTime,
    ) -> QualityResult<()> {
        if !self.status.can_make_decision() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        self.status = InspectionLotStatus::Decided;
        self.decision = Some(decision);
        self.decision_reason = reason;
        self.decided_by = Some(operator);
        self.decided_at = Some(now);

        Ok(())
    }

    /// 关闭检验批。
    pub fn close(&mut self, now: OffsetDateTime) -> QualityResult<()> {
        if self.status != InspectionLotStatus::Decided {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        self.status = InspectionLotStatus::Closed;
        self.closed_at = Some(now);

        Ok(())
    }

    /// 取消检验批。
    pub fn cancel(&mut self, now: OffsetDateTime) -> QualityResult<()> {
        if self.status.is_terminal() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        self.status = InspectionLotStatus::Cancelled;
        self.closed_at = Some(now);

        Ok(())
    }
}

/// 创建检验批输入。
///
/// 这个结构是领域层输入，不是 HTTP DTO。
/// HTTP DTO 后面会放到 interface 层。
#[derive(Debug, Clone)]
pub struct CreateInspectionLot {
    pub id: InspectionLotId,
    pub lot_type: InspectionLotType,
    pub material_id: MaterialId,
    pub batch_number: BatchNumber,
    pub source_transaction_id: Option<String>,
    pub source_doc: Option<String>,
    pub quantity: Decimal,
    pub sample_qty: Decimal,
    pub created_by: Operator,
    pub now: OffsetDateTime,
    pub remark: Option<String>,
}

/// 更新检验批基础信息输入。
#[derive(Debug, Clone)]
pub struct UpdateInspectionLotDetails {
    pub source_transaction_id: Option<String>,
    pub source_doc: Option<String>,
    pub quantity: Decimal,
    pub sample_qty: Decimal,
    pub remark: Option<String>,
}
