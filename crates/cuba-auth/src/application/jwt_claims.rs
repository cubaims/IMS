//! JWT 验签逻辑。
//!
//! `JwtClaims` 数据结构在 `crate::domain::jwt_claims`,本文件只承载
//! 验签函数与错误类型,避免之前 `domain` / `application` 双份定义。

use crate::domain::JwtClaims;
use jsonwebtoken::{DecodingKey, Validation, decode, errors::ErrorKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("token expired")]
    Expired,
    #[error("invalid signature")]
    BadSignature,
    #[error("invalid issuer")]
    BadIssuer,
    #[error("invalid token: {0}")]
    Invalid(String),
}

/// 验证一个 access token,返回解码后的 claims。
///
/// 校验项:
/// - HS256 签名(使用 `secret`)
/// - exp 过期时间(jsonwebtoken 默认开启)
/// - iss 与 `expected_issuer` 严格匹配
///
/// 本函数不查用户表或权限表。IMS 当前采用短期自包含 access token:
/// 禁用用户和权限撤销在登录/refresh 时查库生效,已签发的 access token
/// 最长保留到自身过期时间。
pub fn verify_access_token(
    token: &str,
    secret: &str,
    expected_issuer: &str,
) -> Result<JwtClaims, VerifyError> {
    let mut validation = Validation::default(); // 默认 HS256 + 验 exp
    validation.set_issuer(&[expected_issuer]);

    let claims = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|d| d.claims)
    .map_err(|e| match e.kind() {
        ErrorKind::ExpiredSignature => VerifyError::Expired,
        ErrorKind::InvalidSignature => VerifyError::BadSignature,
        ErrorKind::InvalidIssuer => VerifyError::BadIssuer,
        _ => VerifyError::Invalid(e.to_string()),
    })?;

    if claims.token_type != "access" {
        return Err(VerifyError::Invalid("token type is not access".to_string()));
    }

    Ok(claims)
}
