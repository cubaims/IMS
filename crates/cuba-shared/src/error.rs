use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

/// 应用层统一错误类型。
///
/// 设计要点：
/// - `Database` 不再实现 `From<sqlx::Error>`(去掉了 `#[from]`)。任何
///   sqlx 错误必须显式经过 `map_inventory_db_error` /
///   `map_production_db_error` 转成结构化业务码,避免直接用 `?` 把裸
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

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

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
        Self::Database(err)
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::PermissionDenied(_) => "PERMISSION_DENIED",
            Self::Business { code, .. } => code,
            Self::Database(_) => "DATABASE_ERROR",
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
                | "MRP_MATERIAL_NOT_FOUND_OR_INACTIVE" => StatusCode::NOT_FOUND,

                // Bad request
                "MRP_DEMAND_INVALID"
                | "MRP_VARIANT_REQUIRED"
                | "MRP_QUERY_INVALID"
                | "REPORT_QUERY_INVALID"
                | "REPORT_FORMAT_UNSUPPORTED"
                | "PO_RECEIPT_QTY_EXCEEDED"
                | "SO_SHIPMENT_QTY_EXCEEDED"
                | "INVALID_MOVEMENT_TYPE" => StatusCode::BAD_REQUEST,

                // Conflict
                "INSUFFICIENT_STOCK"
                | "INSUFFICIENT_BATCH_STOCK"
                | "INSUFFICIENT_BIN_STOCK"
                | "NO_AVAILABLE_BATCH"
                | "BIN_CAPACITY_EXCEEDED"
                | "PO_STATUS_INVALID"
                | "SO_STATUS_INVALID"
                | "MRP_SUGGESTION_STATUS_INVALID"
                | "MRP_BUSINESS_RULE_VIOLATION" => StatusCode::CONFLICT,

                // Server-side operation failed
                "MRP_RUN_FAILED"
                | "REPORT_REFRESH_FAILED"
                | "REPORT_EXPORT_FAILED"
                | "DATA_CONSISTENCY_FAILED"
                | "MATERIALIZED_VIEW_REFRESH_FAILED" => StatusCode::INTERNAL_SERVER_ERROR,

                _ => StatusCode::CONFLICT,
            },
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
            Self::Database(_) => "数据库错误,请稍后再试".to_string(),
            Self::Internal(_) => "服务内部错误".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 把内部敏感细节落到日志,不外露给响应
        match &self {
            Self::Database(err) => {
                tracing::error!(error = ?err, "database error");
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
