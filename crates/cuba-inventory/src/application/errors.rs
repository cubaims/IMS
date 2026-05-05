use thiserror::Error;

use crate::domain::InventoryCountDomainError;

/// 盘点应用层错误
///
/// 这里放 Use Case / Repository 可能出现的业务错误。
/// 后续 interface 层再映射成统一 HTTP 错误响应。
#[derive(Debug, Error)]
pub enum InventoryCountApplicationError {
    #[error("盘点单不存在")]
    CountNotFound,

    #[error("盘点明细行不存在")]
    CountLineNotFound,

    #[error("盘点单状态不允许当前操作")]
    StatusInvalid,

    #[error("盘点范围无效")]
    ScopeInvalid,

    #[error("同一范围存在未关闭盘点单")]
    DuplicatedScope,

    #[error("盘点单没有明细")]
    NoLines,

    #[error("盘点明细尚未录入实盘数量")]
    LineNotCounted,

    #[error("差异行必须填写差异原因")]
    ReasonRequired,

    #[error("盘点单已过账")]
    AlreadyPosted,

    #[error("盘点单已关闭")]
    AlreadyClosed,

    #[error("盘点单已取消")]
    Cancelled,

    #[error("实盘数量无效")]
    CountedQtyInvalid,

    #[error("盘点差异过账失败: {0}")]
    DifferencePostFailed(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("领域规则错误: {0}")]
    Domain(#[from] InventoryCountDomainError),
}

impl InventoryCountApplicationError {
    /// 便于 infrastructure 层把 sqlx 错误转换为应用层错误
    pub fn database<E: std::fmt::Display>(err: E) -> Self {
        Self::Database(err.to_string())
    }
}