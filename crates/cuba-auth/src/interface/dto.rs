use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 健康检查响应
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub module: &'static str,
    pub status: &'static str,
}

/// 登录请求 DTO
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应 DTO
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfoDto,
}

/// 用户信息 DTO(同时用作 `me` 接口的响应)
#[derive(Debug, Serialize)]
pub struct UserInfoDto {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}
