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
        cuba_shared::AppError::Internal(format!("盘点模块错误: {}", err))
    }
}
