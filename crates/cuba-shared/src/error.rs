use thiserror::Error;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("JWT error: {0}")]
    Jwt(String),

    #[error("Password hash error: {0}")]
    PasswordHash(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "PERMISSION_DENIED", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", msg),
            AppError::Jwt(msg) => (StatusCode::UNAUTHORIZED, "TOKEN_INVALID", msg),
            AppError::PasswordHash(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "PASSWORD_HASH_FAILED", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        let body = json!({
            "success": false,
            "error_code": error_code,
            "message": message,
            "trace_id": "trace-001"
        });

        (status, axum::Json(body)).into_response()
    }
}