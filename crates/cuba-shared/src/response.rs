use axum::{Json, response::IntoResponse};
use serde::Serialize;

/// 统一 API 响应格式。
///
/// 注意：trace_id 已**不再**在 body 内携带。它由 tower-http 的
/// `SetRequestIdLayer` / `PropagateRequestIdLayer` 通过 `x-request-id`
/// 响应头透传,前端应从响应头读取该字段做链路关联。
///
/// 之前 body 里的 `trace_id` 字段每次都是新的随机 UUID,与
/// 请求上的 `x-request-id` 没有任何关联,等同于无效字段,因此移除。
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 业务成功 + 默认 message="OK"。
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some("OK".to_string()),
        }
    }

    /// 业务成功,不带 message。
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    /// 业务成功,自定义 message。
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
        }
    }
}

impl ApiResponse<()> {
    /// 仅返回 message,不带 data。
    pub fn ok_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            message: Some(message.into()),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::OK, Json(self)).into_response()
    }
}
