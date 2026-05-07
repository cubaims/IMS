//! cuba-shared：所有业务模块共享的核心组件
//!
//! 包含：错误处理、响应格式、认证上下文、分页、配置、数据库错误映射等通用功能。

pub mod config;
pub mod context;
pub mod db_error;
pub mod error;
pub mod pagination;
pub mod response;
pub mod state;
pub mod settings;
// ====================== 公开导出 ======================

pub use config::Settings;
pub use context::CurrentUser;
pub use db_error::{map_inventory_db_error, map_production_db_error};
pub use error::{AppError, AppResult};
pub use pagination::*;
pub use response::ApiResponse;
pub use state::AppState;
