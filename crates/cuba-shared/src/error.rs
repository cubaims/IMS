use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct DatabaseError(pub(crate) sqlx::Error);

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// 应用层统一错误类型。
///
/// 设计要点：
/// - `Database` 不再实现 `From<sqlx::Error>`(去掉了 `#[from]`),且外部
///   crate 不能直接构造。任何 sqlx 错误必须显式经过 `map_*_db_error`
///   转成结构化业务码,避免直接用 `?` 把裸
///   sqlx 错误以"DATABASE_ERROR"形式吐给客户端。
/// - 对外的 `public_message`:`Database` 与 `Internal` 一律返回
///   通用文案,真实错误细节通过 `tracing::error!` 进入日志,**绝不**经
///   响应体外漏。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("business conflict: {code} - {message}")]
    Business { code: &'static str, message: String },

    #[error("database error: {source}")]
    Database { source: DatabaseError },

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    success: bool,
    error_code: &'static str,
    message: String,
}

impl AppError {
    pub fn business(code: &'static str, message: impl Into<String>) -> Self {
        Self::Business {
            code,
            message: message.into(),
        }
    }

    /// 构造未分类的数据库错误。仅在 `map_*_db_error` 内部兜底使用。
    pub fn raw_database(err: sqlx::Error) -> Self {
        Self::Database {
            source: DatabaseError(err),
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::PermissionDenied(_) => "PERMISSION_DENIED",
            Self::Business { code, .. } => code,
            Self::Database { .. } => "DATABASE_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::PermissionDenied(_) => StatusCode::FORBIDDEN,
            Self::Business { code, .. } => match *code {
                // Not found
                "MRP_RUN_NOT_FOUND"
                | "MRP_SUGGESTION_NOT_FOUND"
                | "MRP_VARIANT_NOT_FOUND"
                | "MRP_MATERIAL_NOT_FOUND_OR_INACTIVE"
                | "INSPECTION_LOT_NOT_FOUND"
                | "QUALITY_NOTIFICATION_NOT_FOUND"
                | "TRACE_BATCH_NOT_FOUND"
                | "TRACE_SERIAL_NOT_FOUND" => StatusCode::NOT_FOUND,

                // Bad request
                "MRP_DEMAND_INVALID"
                | "MRP_VARIANT_REQUIRED"
                | "MRP_QUERY_INVALID"
                | "REPORT_QUERY_INVALID"
                | "REPORT_FORMAT_UNSUPPORTED"
                | "INVENTORY_COUNT_SCOPE_INVALID"
                | "INVENTORY_COUNT_LINE_NOT_COUNTED"
                | "INVENTORY_COUNT_REASON_REQUIRED"
                | "COUNTED_QTY_INVALID"
                | "PO_RECEIPT_QTY_EXCEEDED"
                | "SO_SHIPMENT_QTY_EXCEEDED"
                | "INVALID_MOVEMENT_TYPE" => StatusCode::BAD_REQUEST,

                // Capability not exposed yet
                "NOT_IMPLEMENTED" => StatusCode::NOT_IMPLEMENTED,

                // Conflict
                "INSUFFICIENT_STOCK"
                | "INSUFFICIENT_BATCH_STOCK"
                | "INSUFFICIENT_BIN_STOCK"
                | "NO_AVAILABLE_BATCH"
                | "BIN_CAPACITY_EXCEEDED"
                | "BATCH_FROZEN"
                | "BATCH_SCRAPPED"
                | "PO_STATUS_INVALID"
                | "SO_STATUS_INVALID"
                | "MRP_SUGGESTION_STATUS_INVALID"
                | "MRP_BUSINESS_RULE_VIOLATION" => StatusCode::CONFLICT,

                // Server-side operation failed
                "MRP_RUN_FAILED"
                | "PRODUCTION_LOCK_ERROR"
                | "REPORT_QUERY_FAILED"
                | "REPORT_REFRESH_FAILED"
                | "REPORT_EXPORT_FAILED"
                | "TRACE_QUERY_FAILED"
                | "DATA_CONSISTENCY_FAILED"
                | "MATERIALIZED_VIEW_REFRESH_FAILED"
                | "COUNT_DIFFERENCE_POST_FAILED"
                | "COUNT_GAIN_POST_FAILED"
                | "COUNT_LOSS_POST_FAILED"
                | "INVENTORY_COUNT_DATABASE_ERROR" => StatusCode::INTERNAL_SERVER_ERROR,

                // 把这一段加到 Self::Business { code, .. } => match *code { ... } 里
                // 位置随便,但建议放在最末尾"_ => StatusCode::CONFLICT" 之前

                // ===== Master Data: NotFound =====
                "MATERIAL_NOT_FOUND"
                | "BIN_NOT_FOUND"
                | "BATCH_NOT_FOUND"
                | "PO_NOT_FOUND"
                | "PO_LINE_NOT_FOUND"
                | "SO_NOT_FOUND"
                | "SO_LINE_NOT_FOUND"
                | "INVENTORY_COUNT_NOT_FOUND"
                | "INVENTORY_COUNT_LINE_NOT_FOUND"
                | "PRODUCT_VARIANT_NOT_FOUND"
                | "SUPPLIER_NOT_FOUND"
                | "CUSTOMER_NOT_FOUND"
                | "VARIANT_NOT_FOUND"
                | "BOM_NOT_FOUND"
                | "BOM_COMPONENT_NOT_FOUND"
                | "WORK_CENTER_NOT_FOUND"
                | "INSPECTION_CHAR_NOT_FOUND"
                | "DEFECT_CODE_NOT_FOUND" => StatusCode::NOT_FOUND,

                // ===== Master Data: Bad Request(字段非法,但走了业务码路径) =====
                "BIN_CAPACITY_INVALID" | "INSPECTION_LIMIT_INVALID" => StatusCode::BAD_REQUEST,

                // ===== Master Data: Conflict =====
                "DUPLICATE_RECORD"
                | "MATERIAL_ALREADY_EXISTS"
                | "INVENTORY_COUNT_STATUS_INVALID"
                | "INVENTORY_COUNT_DUPLICATED_SCOPE"
                | "INVENTORY_COUNT_ALREADY_POSTED"
                | "INVENTORY_COUNT_ALREADY_CLOSED"
                | "INVENTORY_COUNT_CANCELLED"
                | "INVENTORY_COUNT_NO_LINES"
                | "MATERIAL_INACTIVE"
                | "MATERIAL_HAS_STOCK"
                | "BIN_ALREADY_EXISTS"
                | "BIN_INACTIVE"
                | "BIN_HAS_STOCK"
                | "SUPPLIER_ALREADY_EXISTS"
                | "SUPPLIER_INACTIVE"
                | "PRIMARY_SUPPLIER_ALREADY_EXISTS"
                | "CUSTOMER_ALREADY_EXISTS"
                | "CUSTOMER_INACTIVE"
                | "VARIANT_ALREADY_EXISTS"
                | "VARIANT_INACTIVE"
                | "BOM_ALREADY_EXISTS"
                | "BOM_COMPONENT_DUPLICATED"
                | "BOM_SELF_REFERENCE"
                | "BOM_CYCLE_DETECTED"
                | "BOM_NO_COMPONENTS"
                | "FINISHED_BATCH_ALREADY_EXISTS"
                | "WORK_CENTER_ALREADY_EXISTS"
                | "WORK_CENTER_INACTIVE"
                | "INSPECTION_CHAR_ALREADY_EXISTS"
                | "DEFECT_CODE_ALREADY_EXISTS"
                | "DEFECT_CODE_INACTIVE" => StatusCode::CONFLICT,

                _ => StatusCode::CONFLICT,
            },
            Self::Database { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 暴露给 HTTP 客户端的可读消息。
    /// `Database` / `Internal` **必须**返回通用文案,真实错误进日志。
    pub fn public_message(&self) -> String {
        match self {
            Self::Validation(message)
            | Self::NotFound(message)
            | Self::Unauthorized(message)
            | Self::PermissionDenied(message) => message.clone(),
            Self::Business { message, .. } => message.clone(),
            Self::Database { .. } => "数据库错误,请稍后再试".to_string(),
            Self::Internal(_) => "服务内部错误".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 把内部敏感细节落到日志,不外露给响应
        match &self {
            Self::Database { source } => {
                tracing::error!(error = ?source, "database error");
            }
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
            }
            _ => {}
        }

        let body = ErrorBody {
            success: false,
            error_code: self.error_code(),
            message: self.public_message(),
        };

        (self.http_status(), Json(body)).into_response()
    }
}
