pub mod commands;
pub mod common;
pub mod errors;
pub mod inventory_count_model;
pub mod inventory_count_repository;
pub mod inventory_count_service;
pub mod ports;
pub mod queries;
pub mod services;

pub use common::{Page, PageQuery};
pub use errors::InventoryCountApplicationError;

pub use inventory_count_model::{
    ApproveInventoryCountInput, BatchUpdateInventoryCountLineItem,
    BatchUpdateInventoryCountLinesInput, CancelInventoryCountInput, CloseInventoryCountInput,
    CreateInventoryCountInput, GenerateInventoryCountLinesInput, GetInventoryCountInput,
    InventoryCountScopeFilter, ListInventoryCountsInput, PostInventoryCountInput,
    SubmitInventoryCountInput, UpdateInventoryCountLineInput,
};

pub use inventory_count_repository::{InventoryCountRepository, InventoryCountSummary};

pub use commands::*;
pub use ports::*;
pub use queries::*;
pub use services::*;

impl From<crate::application::InventoryCountApplicationError> for cuba_shared::AppError {
    fn from(err: crate::application::InventoryCountApplicationError) -> Self {
        use crate::application::InventoryCountApplicationError as Error;

        match err {
            Error::CountNotFound => {
                cuba_shared::AppError::business("INVENTORY_COUNT_NOT_FOUND", "盘点单不存在")
            }
            Error::CountLineNotFound => cuba_shared::AppError::business(
                "INVENTORY_COUNT_LINE_NOT_FOUND",
                "盘点明细行不存在",
            ),
            Error::StatusInvalid => cuba_shared::AppError::business(
                "INVENTORY_COUNT_STATUS_INVALID",
                "盘点单状态不允许当前操作",
            ),
            Error::ScopeInvalid => {
                cuba_shared::AppError::business("INVENTORY_COUNT_SCOPE_INVALID", "盘点范围无效")
            }
            Error::DuplicatedScope => cuba_shared::AppError::business(
                "INVENTORY_COUNT_DUPLICATED_SCOPE",
                "同一范围存在未关闭盘点单",
            ),
            Error::NoLines => {
                cuba_shared::AppError::business("INVENTORY_COUNT_NO_LINES", "盘点单没有明细")
            }
            Error::LineNotCounted => cuba_shared::AppError::business(
                "INVENTORY_COUNT_LINE_NOT_COUNTED",
                "盘点明细尚未录入实盘数量",
            ),
            Error::ReasonRequired => cuba_shared::AppError::business(
                "INVENTORY_COUNT_REASON_REQUIRED",
                "差异行必须填写差异原因",
            ),
            Error::AlreadyPosted => {
                cuba_shared::AppError::business("INVENTORY_COUNT_ALREADY_POSTED", "盘点单已过账")
            }
            Error::AlreadyClosed => {
                cuba_shared::AppError::business("INVENTORY_COUNT_ALREADY_CLOSED", "盘点单已关闭")
            }
            Error::Cancelled => {
                cuba_shared::AppError::business("INVENTORY_COUNT_CANCELLED", "盘点单已取消")
            }
            Error::CountedQtyInvalid => {
                cuba_shared::AppError::business("COUNTED_QTY_INVALID", "实盘数量无效")
            }
            Error::DifferencePostFailed(message) => {
                cuba_shared::AppError::business("COUNT_DIFFERENCE_POST_FAILED", message)
            }
            Error::Database(message) => cuba_shared::AppError::business(
                "INVENTORY_COUNT_DATABASE_ERROR",
                format!("盘点数据库操作失败: {message}"),
            ),
            Error::Domain(domain_err) => match domain_err {
                crate::domain::InventoryCountDomainError::StatusInvalid => {
                    cuba_shared::AppError::business(
                        "INVENTORY_COUNT_STATUS_INVALID",
                        "盘点单状态不允许当前操作",
                    )
                }
                crate::domain::InventoryCountDomainError::ScopeInvalid => {
                    cuba_shared::AppError::business("INVENTORY_COUNT_SCOPE_INVALID", "盘点范围无效")
                }
                crate::domain::InventoryCountDomainError::CountedQtyInvalid => {
                    cuba_shared::AppError::business("COUNTED_QTY_INVALID", "实盘数量无效")
                }
                crate::domain::InventoryCountDomainError::DifferenceReasonRequired => {
                    cuba_shared::AppError::business(
                        "INVENTORY_COUNT_REASON_REQUIRED",
                        "差异行必须填写差异原因",
                    )
                }
                crate::domain::InventoryCountDomainError::LineNotCounted => {
                    cuba_shared::AppError::business(
                        "INVENTORY_COUNT_LINE_NOT_COUNTED",
                        "盘点明细尚未录入实盘数量",
                    )
                }
                crate::domain::InventoryCountDomainError::EmptyCountLines => {
                    cuba_shared::AppError::business("INVENTORY_COUNT_NO_LINES", "盘点单没有明细")
                }
                crate::domain::InventoryCountDomainError::DifferenceLineNotPosted => {
                    cuba_shared::AppError::business(
                        "COUNT_DIFFERENCE_POST_FAILED",
                        "存在未完成过账的差异行",
                    )
                }
            },
        }
    }
}
