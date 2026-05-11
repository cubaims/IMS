use thiserror::Error;

/// 质量模块领域错误。
///
/// 这里的错误是业务错误，不是数据库错误。
/// 数据库错误后面会在 infrastructure 层映射成这些业务错误。
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QualityError {
    #[error("检验批不存在")]
    InspectionLotNotFound,

    #[error("检验批状态不允许当前操作")]
    InspectionLotStatusInvalid,

    #[error("检验批已经存在")]
    InspectionLotAlreadyExists,

    #[error("检验结果不能为空")]
    InspectionResultRequired,

    #[error("检验结果无效")]
    InspectionResultInvalid,

    #[error("检验特性不存在")]
    InspectionCharNotFound,

    #[error("检验特性已停用")]
    InspectionCharInactive,

    #[error("不良代码不存在")]
    DefectCodeNotFound,

    #[error("不良代码已停用")]
    DefectCodeInactive,

    #[error("失败的检验结果必须填写不良代码")]
    DefectCodeRequired,

    #[error("质量判定无效")]
    QualityDecisionInvalid,

    #[error("质量判定原因不能为空")]
    QualityDecisionReasonRequired,

    #[error("质量通知不存在")]
    QualityNotificationNotFound,

    #[error("质量通知状态不允许当前操作")]
    QualityNotificationStatusInvalid,

    #[error("批次质量状态无效")]
    BatchQualityStatusInvalid,

    #[error("批次已经冻结")]
    BatchAlreadyFrozen,

    #[error("批次未冻结，不能解冻")]
    BatchNotFrozen,

    #[error("批次已经报废")]
    BatchAlreadyScrapped,

    #[error("批次质量状态禁止出库")]
    BatchQualityBlocked,

    #[error("批次仍处于待检状态")]
    BatchPendingInspection,

    #[error("批次被冻结")]
    BatchFrozen,

    #[error("批次已报废")]
    BatchScrapped,

    #[error("必填字段为空：{0}")]
    RequiredFieldEmpty(&'static str),

    #[error("数量必须大于 0")]
    QuantityMustBePositive,

    #[error("样本数量不能大于检验数量")]
    SampleQtyExceeded,

    #[error("业务规则校验失败：{0}")]
    BusinessRuleViolation(String),
}

/// 质量模块统一 Result。
pub type QualityResult<T> = Result<T, QualityError>;
