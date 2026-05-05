use serde::{Deserialize, Serialize};

/// 检验批类型。
///
/// MVP 先支持：
/// - 采购入库检验
/// - 生产入库检验
/// - 手工检验
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InspectionLotType {
    /// 采购入库检验
    PurchaseReceipt,

    /// 生产入库检验
    ProductionReceipt,

    /// 库存复检，后续可用
    StockRecheck,

    /// 客退检验，二期
    CustomerReturn,

    /// 手工检验
    Manual,
}

/// 检验批状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InspectionLotStatus {
    /// 已创建
    Created,

    /// 检验中
    InProgress,

    /// 检验结果已录入
    ResultEntered,

    /// 已完成质量判定
    Decided,

    /// 已关闭
    Closed,

    /// 已取消
    Cancelled,
}

impl InspectionLotStatus {
    /// 是否允许录入检验结果。
    pub fn can_enter_result(self) -> bool {
        matches!(self, Self::Created | Self::InProgress)
    }

    /// 是否允许提交检验结果。
    pub fn can_submit_result(self) -> bool {
        matches!(self, Self::Created | Self::InProgress)
    }

    /// 是否允许做质量判定。
    pub fn can_make_decision(self) -> bool {
        matches!(self, Self::InProgress | Self::ResultEntered)
    }

    /// 是否是终态。
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Cancelled)
    }
}

/// 单项检验结果状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InspectionResultStatus {
    /// 合格
    Pass,

    /// 不合格
    Fail,

    /// 不适用
    NotApplicable,
}

/// 质量判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InspectionDecision {
    /// 接收，批次转为合格
    Accept,

    /// 冻结，批次转为冻结
    Freeze,

    /// 报废，批次转为报废
    Scrap,
}

/// 批次质量状态。
///
/// 注意：这里为了贴近数据库和业务口径，序列化后使用中文。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchQualityStatus {
    /// 待检
    #[serde(rename = "待检")]
    PendingInspection,

    /// 合格
    #[serde(rename = "合格")]
    Qualified,

    /// 冻结
    #[serde(rename = "冻结")]
    Frozen,

    /// 报废
    #[serde(rename = "报废")]
    Scrapped,
}

impl BatchQualityStatus {
    /// 是否允许出库类动作。
    ///
    /// MVP 规则：
    /// 只有“合格”批次允许销售发货、生产领料、手工 261 出库。
    pub fn can_outbound(self) -> bool {
        matches!(self, Self::Qualified)
    }

    /// 是否允许冻结。
    pub fn can_freeze(self) -> bool {
        matches!(self, Self::PendingInspection | Self::Qualified)
    }

    /// 是否允许解冻。
    pub fn can_unfreeze(self) -> bool {
        matches!(self, Self::Frozen)
    }

    /// 是否允许转报废。
    pub fn can_scrap(self) -> bool {
        matches!(
            self,
            Self::PendingInspection | Self::Qualified | Self::Frozen
        )
    }
}

impl InspectionDecision {
    /// 质量判定对应的批次质量状态。
    pub fn target_batch_status(self) -> BatchQualityStatus {
        match self {
            Self::Accept => BatchQualityStatus::Qualified,
            Self::Freeze => BatchQualityStatus::Frozen,
            Self::Scrap => BatchQualityStatus::Scrapped,
        }
    }
}

/// 质量通知状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityNotificationStatus {
    /// 已创建
    Open,

    /// 处理中
    InProgress,

    /// 已解决
    Resolved,

    /// 已关闭
    Closed,

    /// 已取消
    Cancelled,
}

impl QualityNotificationStatus {
    /// 是否允许修改。
    pub fn can_update(self) -> bool {
        !matches!(self, Self::Closed | Self::Cancelled)
    }

    /// 是否允许关闭。
    pub fn can_close(self) -> bool {
        matches!(self, Self::Resolved)
    }
}

/// 质量通知严重等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityNotificationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 批次质量动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BatchQualityAction {
    /// 创建检验批后进入待检
    MarkPendingInspection,

    /// 判定合格
    Accept,

    /// 冻结批次
    Freeze,

    /// 解冻批次
    Unfreeze,

    /// 标记质量报废
    Scrap,
}