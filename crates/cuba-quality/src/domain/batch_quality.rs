use crate::domain::{
    BatchNumber, BatchQualityAction, BatchQualityStatus, Operator, QualityError, QualityResult,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// 批次质量状态快照。
///
/// 注意：这个对象不代表完整库存批次，只代表质量模块关心的部分。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQuality {
    pub batch_number: BatchNumber,
    pub status: BatchQualityStatus,
}

impl BatchQuality {
    /// 创建批次质量状态。
    pub fn new(batch_number: BatchNumber, status: BatchQualityStatus) -> Self {
        Self {
            batch_number,
            status,
        }
    }

    /// 冻结批次。
    pub fn freeze(&mut self) -> QualityResult<BatchQualityStatusChanged> {
        if self.status == BatchQualityStatus::Scrapped {
            return Err(QualityError::BatchAlreadyScrapped);
        }

        if self.status == BatchQualityStatus::Frozen {
            return Err(QualityError::BatchAlreadyFrozen);
        }

        if !self.status.can_freeze() {
            return Err(QualityError::BatchQualityStatusInvalid);
        }

        let old_status = self.status;
        self.status = BatchQualityStatus::Frozen;

        Ok(BatchQualityStatusChanged {
            batch_number: self.batch_number.clone(),
            old_status,
            new_status: self.status,
            action: BatchQualityAction::Freeze,
            reason: None,
            reference_doc: None,
            operator: None,
            occurred_at: None,
            remark: None,
        })
    }

    /// 解冻批次。
    pub fn unfreeze(
        &mut self,
        target_status: BatchQualityStatus,
    ) -> QualityResult<BatchQualityStatusChanged> {
        if !self.status.can_unfreeze() {
            return Err(QualityError::BatchNotFrozen);
        }

        if !matches!(
            target_status,
            BatchQualityStatus::Qualified | BatchQualityStatus::PendingInspection
        ) {
            return Err(QualityError::BatchQualityStatusInvalid);
        }

        let old_status = self.status;
        self.status = target_status;

        Ok(BatchQualityStatusChanged {
            batch_number: self.batch_number.clone(),
            old_status,
            new_status: self.status,
            action: BatchQualityAction::Unfreeze,
            reason: None,
            reference_doc: None,
            operator: None,
            occurred_at: None,
            remark: None,
        })
    }

    /// 标记质量报废。
    ///
    /// 注意：
    /// 这里只是质量报废，不扣减库存。
    /// 实际库存扣减应走库存模块的 999 报废出库。
    pub fn scrap(&mut self) -> QualityResult<BatchQualityStatusChanged> {
        if self.status == BatchQualityStatus::Scrapped {
            return Err(QualityError::BatchAlreadyScrapped);
        }

        if !self.status.can_scrap() {
            return Err(QualityError::BatchQualityStatusInvalid);
        }

        let old_status = self.status;
        self.status = BatchQualityStatus::Scrapped;

        Ok(BatchQualityStatusChanged {
            batch_number: self.batch_number.clone(),
            old_status,
            new_status: self.status,
            action: BatchQualityAction::Scrap,
            reason: None,
            reference_doc: None,
            operator: None,
            occurred_at: None,
            remark: None,
        })
    }

    fn ensure_context_reason(context: &BatchQualityChangeContext) -> QualityResult<()> {
        if context.reason.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("reason"));
        }

        Ok(())
    }

    fn attach_context(
        mut event: BatchQualityStatusChanged,
        context: BatchQualityChangeContext,
    ) -> BatchQualityStatusChanged {
        event.reason = Some(context.reason.trim().to_string());
        event.reference_doc = context.reference_doc;
        event.operator = Some(context.operator);
        event.occurred_at = Some(context.occurred_at);
        event.remark = context.remark;
        event
    }

    /// 带上下文冻结批次。
    ///
    /// 新 application 层建议使用这个方法，这样可以直接把事件写入批次历史。
    pub fn freeze_with_context(
        &mut self,
        context: BatchQualityChangeContext,
    ) -> QualityResult<BatchQualityStatusChanged> {
        Self::ensure_context_reason(&context)?;
        let event = self.freeze()?;
        Ok(Self::attach_context(event, context))
    }

    /// 带上下文解冻批次。
    pub fn unfreeze_with_context(
        &mut self,
        target_status: BatchQualityStatus,
        context: BatchQualityChangeContext,
    ) -> QualityResult<BatchQualityStatusChanged> {
        Self::ensure_context_reason(&context)?;
        let event = self.unfreeze(target_status)?;
        Ok(Self::attach_context(event, context))
    }

    /// 带上下文标记质量报废。
    pub fn scrap_with_context(
        &mut self,
        context: BatchQualityChangeContext,
    ) -> QualityResult<BatchQualityStatusChanged> {
        Self::ensure_context_reason(&context)?;
        let event = self.scrap()?;
        Ok(Self::attach_context(event, context))
    }

    /// 校验是否允许出库。
    pub fn ensure_can_outbound(&self) -> QualityResult<()> {
        match self.status {
            BatchQualityStatus::Qualified => Ok(()),
            BatchQualityStatus::PendingInspection => Err(QualityError::BatchPendingInspection),
            BatchQualityStatus::Frozen => Err(QualityError::BatchFrozen),
            BatchQualityStatus::Scrapped => Err(QualityError::BatchScrapped),
        }
    }
}

/// 批次质量状态变更事件。
///
/// 后续 application 层会把它交给 repository，
/// 写入 wms.wms_batch_history。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQualityStatusChanged {
    pub batch_number: BatchNumber,
    pub old_status: BatchQualityStatus,
    pub new_status: BatchQualityStatus,
    pub action: BatchQualityAction,

    /// 状态变更原因。
    ///
    /// 旧方法 freeze / unfreeze / scrap 可能为空。
    /// 新 application 层建议使用 *_with_context 方法保证必填。
    pub reason: Option<String>,

    /// 来源单据，例如检验批、质量通知、人工操作单。
    pub reference_doc: Option<String>,

    /// 操作人。
    pub operator: Option<Operator>,

    /// 发生时间。
    pub occurred_at: Option<OffsetDateTime>,

    /// 备注。
    pub remark: Option<String>,
}

/// 批次质量状态变更上下文。
///
/// application 层调用冻结、解冻、报废时应传入该上下文，
/// 用于后续写入 wms.wms_batch_history。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQualityChangeContext {
    pub reason: String,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub occurred_at: OffsetDateTime,
    pub remark: Option<String>,
}

/// 批次质量历史。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQualityHistory {
    pub batch_number: BatchNumber,
    pub old_status: Option<BatchQualityStatus>,
    pub new_status: BatchQualityStatus,
    pub action: BatchQualityAction,
    pub reason: String,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub occurred_at: OffsetDateTime,
    pub remark: Option<String>,
}