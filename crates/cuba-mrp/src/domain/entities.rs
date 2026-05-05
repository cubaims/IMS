use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

// =============================================================================
// 错误定义
// =============================================================================

/// MRP 模块领域错误。
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MrpError {
    #[error("MRP 运行记录不存在")]
    MrpRunNotFound,

    #[error("MRP 建议不存在")]
    MrpSuggestionNotFound,

    #[error("MRP 建议状态不允许当前操作")]
    MrpSuggestionStatusInvalid,

    #[error("物料不存在或已停用")]
    MaterialNotFoundOrInactive,

    #[error("产品变体不存在")]
    ProductVariantNotFound,

    #[error("需求数量必须大于 0")]
    DemandQtyMustBePositive,

    #[error("需求日期不能为空")]
    DemandDateRequired,

    #[error("MRP 运行失败")]
    MrpRunFailed,

    #[error("必填字段为空：{0}")]
    RequiredFieldEmpty(&'static str),

    #[error("业务规则校验失败：{0}")]
    BusinessRuleViolation(String),
}

/// MRP 模块统一 Result。
pub type MrpResult<T> = Result<T, MrpError>;

// =============================================================================
// 值对象
// =============================================================================

/// MRP 运行 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MrpRunId(String);

impl MrpRunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// MRP 建议 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MrpSuggestionId(String);

impl MrpSuggestionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 物料 ID。
///
/// 后续如果 cuba-master-data 已有 MaterialId，
/// 可以统一上移到 cuba-shared 或改为复用 master-data 类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(String);

impl MaterialId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 产品变体 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductVariantId(String);

impl ProductVariantId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 操作人。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Operator(String);

impl Operator {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// 枚举
// =============================================================================

/// MRP 运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MrpRunStatus {
    /// 已创建
    Created,

    /// 运行中
    Running,

    /// 已完成
    Completed,

    /// 运行失败
    Failed,
}

/// MRP 建议类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MrpSuggestionType {
    /// 采购建议
    Purchase,

    /// 生产建议
    Production,
}

/// MRP 建议状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MrpSuggestionStatus {
    /// 新建议
    New,

    /// 已确认
    Confirmed,

    /// 已取消
    Cancelled,

    /// 已转换为采购订单或生产订单
    Converted,
}

impl MrpSuggestionStatus {
    /// 是否允许确认。
    pub fn can_confirm(self) -> bool {
        matches!(self, Self::New)
    }

    /// 是否允许取消。
    pub fn can_cancel(self) -> bool {
        matches!(self, Self::New | Self::Confirmed)
    }

    /// 是否允许转换单据。
    pub fn can_convert(self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

// =============================================================================
// MRP 运行聚合
// =============================================================================

/// MRP 运行记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpRun {
    pub id: MrpRunId,

    /// 需求物料。
    ///
    /// 有些 MRP 是按成品或产品变体运行，
    /// 有些是按物料运行。
    pub material_id: Option<MaterialId>,

    /// 产品变体。
    pub product_variant_id: Option<ProductVariantId>,

    /// 需求数量。
    pub demand_qty: Decimal,

    /// 需求日期。
    pub demand_date: OffsetDateTime,

    /// 状态。
    pub status: MrpRunStatus,

    /// 创建人。
    pub created_by: Operator,

    /// 创建时间。
    pub created_at: OffsetDateTime,

    /// 开始时间。
    pub started_at: Option<OffsetDateTime>,

    /// 完成时间。
    pub completed_at: Option<OffsetDateTime>,

    /// 失败原因。
    pub error_message: Option<String>,

    /// 备注。
    pub remark: Option<String>,
}

impl MrpRun {
    /// 创建 MRP 运行记录。
    pub fn create(input: CreateMrpRun) -> MrpResult<Self> {
        if input.demand_qty <= Decimal::ZERO {
            return Err(MrpError::DemandQtyMustBePositive);
        }

        if input.material_id.is_none() && input.product_variant_id.is_none() {
            return Err(MrpError::BusinessRuleViolation(
                "material_id 和 product_variant_id 不能同时为空".to_string(),
            ));
        }

        Ok(Self {
            id: input.id,
            material_id: input.material_id,
            product_variant_id: input.product_variant_id,
            demand_qty: input.demand_qty,
            demand_date: input.demand_date,
            status: MrpRunStatus::Created,
            created_by: input.created_by,
            created_at: input.now,
            started_at: None,
            completed_at: None,
            error_message: None,
            remark: input.remark,
        })
    }

    /// 标记运行中。
    pub fn mark_running(&mut self, now: OffsetDateTime) {
        self.status = MrpRunStatus::Running;
        self.started_at = Some(now);
    }

    /// 标记完成。
    pub fn mark_completed(&mut self, now: OffsetDateTime) {
        self.status = MrpRunStatus::Completed;
        self.completed_at = Some(now);
    }

    /// 标记失败。
    pub fn mark_failed(&mut self, now: OffsetDateTime, error_message: String) {
        self.status = MrpRunStatus::Failed;
        self.completed_at = Some(now);
        self.error_message = Some(error_message);
    }
}

/// 创建 MRP 运行输入。
#[derive(Debug, Clone)]
pub struct CreateMrpRun {
    pub id: MrpRunId,
    pub material_id: Option<MaterialId>,
    pub product_variant_id: Option<ProductVariantId>,
    pub demand_qty: Decimal,
    pub demand_date: OffsetDateTime,
    pub created_by: Operator,
    pub now: OffsetDateTime,
    pub remark: Option<String>,
}

// =============================================================================
// MRP 建议聚合
// =============================================================================

/// MRP 建议。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpSuggestion {
    pub id: MrpSuggestionId,
    pub run_id: MrpRunId,

    /// 建议类型：采购 / 生产。
    pub suggestion_type: MrpSuggestionType,

    /// 建议物料。
    pub material_id: MaterialId,

    /// 建议数量。
    pub suggested_qty: Decimal,

    /// 需求日期。
    pub required_date: OffsetDateTime,

    /// 建议日期。
    pub suggested_date: OffsetDateTime,

    /// 建议供应商，可为空。
    pub supplier_id: Option<String>,

    /// 建议生产工作中心，可为空。
    pub work_center_id: Option<String>,

    /// 状态。
    pub status: MrpSuggestionStatus,

    /// 创建时间。
    pub created_at: OffsetDateTime,

    /// 确认人。
    pub confirmed_by: Option<Operator>,

    /// 确认时间。
    pub confirmed_at: Option<OffsetDateTime>,

    /// 备注。
    pub remark: Option<String>,
}

impl MrpSuggestion {
    /// 确认 MRP 建议。
    pub fn confirm(
        &mut self,
        operator: Operator,
        now: OffsetDateTime,
    ) -> MrpResult<()> {
        if !self.status.can_confirm() {
            return Err(MrpError::MrpSuggestionStatusInvalid);
        }

        self.status = MrpSuggestionStatus::Confirmed;
        self.confirmed_by = Some(operator);
        self.confirmed_at = Some(now);

        Ok(())
    }

    /// 取消 MRP 建议。
    pub fn cancel(&mut self) -> MrpResult<()> {
        if !self.status.can_cancel() {
            return Err(MrpError::MrpSuggestionStatusInvalid);
        }

        self.status = MrpSuggestionStatus::Cancelled;

        Ok(())
    }

    /// 标记为已转换单据。
    pub fn mark_converted(&mut self) -> MrpResult<()> {
        if !self.status.can_convert() {
            return Err(MrpError::MrpSuggestionStatusInvalid);
        }

        self.status = MrpSuggestionStatus::Converted;

        Ok(())
    }
}