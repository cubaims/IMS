use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use cuba_shared::AppError;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub struct IssuedRefreshToken {
    pub token_id: Uuid,
    pub selector: String,
    pub token: String,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
}

pub struct ParsedRefreshToken {
    pub selector: String,
    secret: String,
}

pub fn issue_refresh_token(expires_seconds: i64) -> Result<IssuedRefreshToken, AppError> {
    if expires_seconds <= 0 {
        return Err(AppError::Internal(
            "JWT_REFRESH_EXPIRES_SECONDS must be positive".to_string(),
        ));
    }

    let token_id = Uuid::new_v4();
    let selector = Uuid::new_v4().simple().to_string();
    let secret = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let token = format!("{selector}.{secret}");
    let token_hash = hash_refresh_secret(&secret)?;
    let expires_at = OffsetDateTime::now_utc() + Duration::seconds(expires_seconds);

    Ok(IssuedRefreshToken {
        token_id,
        selector,
        token,
        token_hash,
        expires_at,
    })
}

pub fn parse_refresh_token(token: &str) -> Result<ParsedRefreshToken, AppError> {
    let (selector, secret) = token.split_once('.').ok_or_else(refresh_token_invalid)?;

    if selector.trim().is_empty() || secret.trim().is_empty() {
        return Err(refresh_token_invalid());
    }

    Ok(ParsedRefreshToken {
        selector: selector.to_string(),
        secret: secret.to_string(),
    })
}

pub fn verify_refresh_secret(
    parsed: &ParsedRefreshToken,
    stored_hash: &str,
) -> Result<(), AppError> {
    let hash = PasswordHash::new(stored_hash).map_err(|e| AppError::Internal(e.to_string()))?;

    Argon2::default()
        .verify_password(parsed.secret.as_bytes(), &hash)
        .map_err(|_| refresh_token_invalid())
}

fn hash_refresh_secret(secret: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(e.to_string()))
}

fn refresh_token_invalid() -> AppError {
    AppError::Unauthorized("REFRESH_TOKEN_INVALID".to_string())
}
