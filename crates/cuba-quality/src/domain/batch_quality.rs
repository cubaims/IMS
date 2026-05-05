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
        })
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