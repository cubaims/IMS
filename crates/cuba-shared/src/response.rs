use axum::{Json, response::IntoResponse};
use serde::Serialize;
use uuid::Uuid;

/// 统一 API 响应格式
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub trace_id: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some("OK".to_string()),
            trace_id: Uuid::new_v4().to_string(), // 默认值，会被 middleware 覆盖
        }
    }

    pub fn success(data: T, trace_id: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            trace_id: trace_id.into(),
        }
    }

    pub fn success_with_message(
        data: T,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
            trace_id: trace_id.into(),
        }
    }
}

impl ApiResponse<()> {
    pub fn ok_message(message: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            message: Some(message.into()),
            trace_id: trace_id.into(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::OK, Json(self)).into_response()
    }
}

/// 从 Request 中提取 x-request-id
pub fn extract_trace_id<B>(req: &axum::http::Request<B>) -> String {
    req.headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}
