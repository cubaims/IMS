use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ====================== JWT Claims ======================

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,           // user_id
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub exp: usize,          // 过期时间（unix timestamp）
    pub iat: usize,          // 签发时间
    pub iss: String,         // 签发者
}

// ====================== JWT 验证逻辑 ======================

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("token 已过期")]
    Expired,
    #[error("token 无效: {0}")]
    Invalid(String),
    #[error("签发者不匹配")]
    BadIssuer,
    #[error("签名无效")]
    BadSignature,
}

pub fn verify_access_token(token: &str, secret: &str) -> Result<JwtClaims, VerifyError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let mut validation = Validation::default();
    validation.validate_exp = true;

    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => VerifyError::Expired,
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => VerifyError::BadIssuer,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => VerifyError::BadSignature,
            _ => VerifyError::Invalid(e.to_string()),
        })
}