use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub password_hash: String, // 绝不暴露给前端
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub role_id: Option<String>,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// 用于登录时返回给前端的用户信息（不含密码）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}
