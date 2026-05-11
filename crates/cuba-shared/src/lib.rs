//! cuba-shared:所有业务模块共享的核心组件
//!
//! 包含:错误处理、响应格式、认证上下文、分页、配置、数据库错误映射、
//! 审计字段等通用功能。

pub mod audit;
pub mod config;
pub mod context;
pub mod db_error;
pub mod error;
pub mod pagination;
pub mod response;
pub mod state;

// ====================== 公开导出 ======================
// 显式导出,避免通配符在多个子模块之间撞名。

pub use audit::{
    AuditCategory, AuditInfo, audit_category_for_action, audit_module_for_event,
    audit_module_for_table, write_audit_change, write_audit_event,
};
pub use config::{ConfigError, Settings, WorkerSettings};
pub use context::CurrentUser;
pub use db_error::{
    map_auth_db_error, map_inventory_db_error, map_master_data_db_error, map_mrp_db_error,
    map_production_db_error, map_purchase_db_error, map_quality_db_error, map_reporting_db_error,
    map_sales_db_error, map_traceability_db_error, map_worker_db_error,
};
pub use error::{AppError, AppResult};
pub use pagination::{Page, PageQuery, SortOrder};
pub use response::ApiResponse;
pub use state::AppState;
