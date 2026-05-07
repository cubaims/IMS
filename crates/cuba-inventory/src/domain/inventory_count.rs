use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// 盘点领域错误
/// 这里先放领域层能直接判断的错误，后续 application 层会继续包装成统一 API 错误。
#[derive(Debug, Error)]
pub enum InventoryCountDomainError {
    #[error("盘点单状态不允许当前操作")]
    StatusInvalid,

    #[error("盘点范围无效")]
    ScopeInvalid,

    #[error("实盘数量不能小于 0")]
    CountedQtyInvalid,

    #[error("差异行必须填写差异原因")]
    DifferenceReasonRequired,

    #[error("盘点行尚未录入实盘数量")]
    LineNotCounted,

    #[error("盘点单没有明细行")]
    EmptyCountLines,

    #[error("存在未完成过账的差异行")]
    DifferenceLineNotPosted,
}

/// 盘点单状态
///
/// 模板要求状态流转：
/// DRAFT -> COUNTING -> SUBMITTED -> APPROVED -> POSTED -> CLOSED
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InventoryCountStatus {
    Draft,
    Counting,
    Submitted,
    Approved,
    Posted,
    Closed,
    Cancelled,
}

impl InventoryCountStatus {
    /// 是否允许生成盘点明细
    pub fn can_generate_lines(&self) -> bool {
        matches!(self, Self::Draft)
    }

    /// 是否允许录入实盘数量
    pub fn can_update_counted_qty(&self) -> bool {
        matches!(self, Self::Counting)
    }

    /// 是否允许提交
    pub fn can_submit(&self) -> bool {
        matches!(self, Self::Counting)
    }

    /// 是否允许审核
    pub fn can_approve(&self) -> bool {
        matches!(self, Self::Submitted)
    }

    /// 是否允许过账
    pub fn can_post(&self) -> bool {
        matches!(self, Self::Approved)
    }

    /// 是否允许关闭
    pub fn can_close(&self) -> bool {
        matches!(self, Self::Posted)
    }

    /// 是否为只读状态
    pub fn is_readonly(&self) -> bool {
        matches!(self, Self::Closed | Self::Cancelled)
    }
}

/// 盘点范围
///
/// MVP 先支持 BIN / MATERIAL / ZONE。
/// FULL / BATCH / CYCLE 预留，后续可逐步开放。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InventoryCountScope {
    Full,
    Zone,
    Bin,
    Material,
    Batch,
    Cycle,
}

impl InventoryCountScope {
    /// MVP 支持的范围
    pub fn is_mvp_supported(&self) -> bool {
        matches!(self, Self::Bin | Self::Material | Self::Zone)
    }
}

/// 盘点业务类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InventoryCountType {
    Regular,
    Cycle,
    Adjustment,
    YearEnd,
}

/// 盘点明细状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InventoryCountLineStatus {
    Pending,
    Counted,
    Posted,
}

/// 盘点差异对应的库存移动类型
///
/// difference_qty > 0 -> 701 盘盈
/// difference_qty < 0 -> 702 盘亏
/// difference_qty = 0 -> None 不过账
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InventoryCountMovementType {
    /// 盘盈
    Gain701,

    /// 盘亏
    Loss702,
}

impl InventoryCountMovementType {
    /// 数据库存储时使用的移动类型编码
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::Gain701 => "701",
            Self::Loss702 => "702",
        }
    }
}

/// 盘点单头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCount {
    pub count_doc_id: String,
    pub count_type: InventoryCountType,
    pub count_scope: InventoryCountScope,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub status: InventoryCountStatus,

    pub created_by: String,
    pub approved_by: Option<String>,
    pub posted_by: Option<String>,

    pub created_at: OffsetDateTime,
    pub approved_at: Option<OffsetDateTime>,
    pub posted_at: Option<OffsetDateTime>,
    pub closed_at: Option<OffsetDateTime>,

    pub remark: Option<String>,

    /// 明细行
    pub lines: Vec<InventoryCountLine>,
}

impl InventoryCount {
    /// 创建一个新的盘点单聚合
    pub fn new(
        count_doc_id: String,
        count_type: InventoryCountType,
        count_scope: InventoryCountScope,
        zone_code: Option<String>,
        bin_code: Option<String>,
        material_id: Option<String>,
        batch_number: Option<String>,
        created_by: String,
        remark: Option<String>,
    ) -> Result<Self, InventoryCountDomainError> {
        // MVP 阶段只开放 BIN / MATERIAL / ZONE
        if !count_scope.is_mvp_supported() {
            return Err(InventoryCountDomainError::ScopeInvalid);
        }

        // 按不同范围校验必填字段
        match count_scope {
            InventoryCountScope::Bin if bin_code.is_none() => {
                return Err(InventoryCountDomainError::ScopeInvalid);
            }
            InventoryCountScope::Material if material_id.is_none() => {
                return Err(InventoryCountDomainError::ScopeInvalid);
            }
            InventoryCountScope::Zone if zone_code.is_none() => {
                return Err(InventoryCountDomainError::ScopeInvalid);
            }
            _ => {}
        }

        Ok(Self {
            count_doc_id,
            count_type,
            count_scope,
            zone_code,
            bin_code,
            material_id,
            batch_number,
            status: InventoryCountStatus::Draft,
            created_by,
            approved_by: None,
            posted_by: None,
            created_at: OffsetDateTime::now_utc(),
            approved_at: None,
            posted_at: None,
            closed_at: None,
            remark,
            lines: Vec::new(),
        })
    }

    /// 生成明细后进入 COUNTING 状态
    pub fn mark_counting(
        &mut self,
        lines: Vec<InventoryCountLine>,
    ) -> Result<(), InventoryCountDomainError> {
        if !self.status.can_generate_lines() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        self.lines = lines;
        self.status = InventoryCountStatus::Counting;
        Ok(())
    }

    /// 提交盘点单
    ///
    /// 规则：
    /// 1. 所有必盘行必须录入 counted_qty
    /// 2. 差异行必须填写 difference_reason
    pub fn submit(&mut self) -> Result<(), InventoryCountDomainError> {
        if !self.status.can_submit() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        if self.lines.is_empty() {
            return Err(InventoryCountDomainError::EmptyCountLines);
        }

        for line in &self.lines {
            line.validate_before_submit()?;
        }

        self.status = InventoryCountStatus::Submitted;
        Ok(())
    }

    /// 审核通过
    pub fn approve(&mut self, approved_by: String) -> Result<(), InventoryCountDomainError> {
        if !self.status.can_approve() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        self.status = InventoryCountStatus::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(OffsetDateTime::now_utc());

        Ok(())
    }

    /// 审核退回，回到 COUNTING
    pub fn reject_to_recount(&mut self) -> Result<(), InventoryCountDomainError> {
        if !self.status.can_approve() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        self.status = InventoryCountStatus::Counting;
        Ok(())
    }

    /// 过账完成后标记为 POSTED
    pub fn mark_posted(&mut self, posted_by: String) -> Result<(), InventoryCountDomainError> {
        if !self.status.can_post() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        let has_unposted_difference_line = self
            .lines
            .iter()
            .any(|line| line.has_difference() && line.status != InventoryCountLineStatus::Posted);

        if has_unposted_difference_line {
            return Err(InventoryCountDomainError::DifferenceLineNotPosted);
        }

        self.status = InventoryCountStatus::Posted;
        self.posted_by = Some(posted_by);
        self.posted_at = Some(OffsetDateTime::now_utc());

        Ok(())
    }

    /// 关闭盘点单
    pub fn close(&mut self) -> Result<(), InventoryCountDomainError> {
        if !self.status.can_close() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        self.status = InventoryCountStatus::Closed;
        self.closed_at = Some(OffsetDateTime::now_utc());

        Ok(())
    }

    /// 取消盘点单
    pub fn cancel(&mut self) -> Result<(), InventoryCountDomainError> {
        if self.status.is_readonly() {
            return Err(InventoryCountDomainError::StatusInvalid);
        }

        self.status = InventoryCountStatus::Cancelled;
        Ok(())
    }
}

/// 盘点明细行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountLine {
    pub count_doc_id: String,
    pub line_no: i32,

    pub material_id: String,
    pub bin_code: String,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,

    /// 系统账面数量，来自 wms.wms_bin_stock.qty 的快照
    pub system_qty: Decimal,

    /// 实盘数量，录入前为空
    pub counted_qty: Option<Decimal>,

    /// 差异数量 = counted_qty - system_qty
    pub difference_qty: Option<Decimal>,

    /// 差异原因，差异行提交前必须填写
    pub difference_reason: Option<String>,

    /// 自动判断：盘盈 701 / 盘亏 702 / 无差异为空
    pub movement_type: Option<InventoryCountMovementType>,

    /// 过账后回写的库存事务 ID
    pub transaction_id: Option<String>,

    pub status: InventoryCountLineStatus,
    pub remark: Option<String>,
}

impl InventoryCountLine {
    /// 根据库存快照创建盘点明细
    pub fn from_stock_snapshot(
        count_doc_id: String,
        line_no: i32,
        material_id: String,
        bin_code: String,
        batch_number: Option<String>,
        quality_status: Option<String>,
        system_qty: Decimal,
    ) -> Self {
        Self {
            count_doc_id,
            line_no,
            material_id,
            bin_code,
            batch_number,
            quality_status,
            system_qty,
            counted_qty: None,
            difference_qty: None,
            difference_reason: None,
            movement_type: None,
            transaction_id: None,
            status: InventoryCountLineStatus::Pending,
            remark: None,
        }
    }

    /// 录入实盘数量，并自动计算差异和移动类型
    pub fn enter_counted_qty(
        &mut self,
        counted_qty: Decimal,
        difference_reason: Option<String>,
        remark: Option<String>,
    ) -> Result<(), InventoryCountDomainError> {
        if counted_qty < Decimal::ZERO {
            return Err(InventoryCountDomainError::CountedQtyInvalid);
        }

        let difference_qty = counted_qty - self.system_qty;
        let movement_type = Self::decide_movement_type(difference_qty);

        self.counted_qty = Some(counted_qty);
        self.difference_qty = Some(difference_qty);
        self.movement_type = movement_type;
        self.difference_reason = difference_reason;
        self.remark = remark;
        self.status = InventoryCountLineStatus::Counted;

        Ok(())
    }

    /// 根据差异数量判断移动类型
    fn decide_movement_type(difference_qty: Decimal) -> Option<InventoryCountMovementType> {
        if difference_qty > Decimal::ZERO {
            Some(InventoryCountMovementType::Gain701)
        } else if difference_qty < Decimal::ZERO {
            Some(InventoryCountMovementType::Loss702)
        } else {
            None
        }
    }

    /// 提交前校验
    pub fn validate_before_submit(&self) -> Result<(), InventoryCountDomainError> {
        let difference_qty = self
            .difference_qty
            .ok_or(InventoryCountDomainError::LineNotCounted)?;

        // 有差异时必须填写原因
        if difference_qty != Decimal::ZERO
            && self
                .difference_reason
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(InventoryCountDomainError::DifferenceReasonRequired);
        }

        Ok(())
    }

    /// 是否有差异
    pub fn has_difference(&self) -> bool {
        self.difference_qty
            .map(|qty| qty != Decimal::ZERO)
            .unwrap_or(false)
    }

    /// 是否是盘盈
    pub fn is_gain(&self) -> bool {
        self.difference_qty
            .map(|qty| qty > Decimal::ZERO)
            .unwrap_or(false)
    }

    /// 是否是盘亏
    pub fn is_loss(&self) -> bool {
        self.difference_qty
            .map(|qty| qty < Decimal::ZERO)
            .unwrap_or(false)
    }

    /// 过账数量
    ///
    /// 盘盈：difference_qty
    /// 盘亏：abs(difference_qty)
    pub fn posting_qty(&self) -> Decimal {
        self.difference_qty.unwrap_or(Decimal::ZERO).abs()
    }

    /// 回写库存事务 ID
    pub fn mark_posted(&mut self, transaction_id: String) {
        self.transaction_id = Some(transaction_id);
        self.status = InventoryCountLineStatus::Posted;
    }
}

/// 盘点过账结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountPostingResult {
    pub count_doc_id: String,
    pub status: InventoryCountStatus,
    pub transactions: Vec<InventoryCountPostedTransaction>,
    pub reports_stale: bool,
}

/// 单行过账生成的库存事务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountPostedTransaction {
    pub line_no: i32,
    pub transaction_id: String,
    pub movement_type: String,
    pub material_id: String,
    pub quantity: Decimal,
    pub from_bin: Option<String>,
    pub to_bin: Option<String>,
    pub batch_number: Option<String>,
}
