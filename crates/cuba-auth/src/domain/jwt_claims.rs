use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid, // user_id
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub exp: usize,  // 过期时间（秒级时间戳）
    pub iat: usize,  // 签发时间
    pub iss: String, // 签发者
}
