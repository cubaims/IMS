use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    /// 绝不暴露给前端
    pub password_hash: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub role_id: Option<String>,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
