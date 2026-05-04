use crate::domain::{JwtClaims, User};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::{Duration, Utc};
use cuba_shared::{AppError, CurrentUser};
use jsonwebtoken::{EncodingKey, Header, encode};
use tracing::info;

pub struct LoginUseCase {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_expires_seconds: i64,
}

impl LoginUseCase {
    pub fn new(jwt_secret: String, jwt_issuer: String, jwt_expires_seconds: i64) -> Self {
        Self {
            jwt_secret,
            jwt_issuer,
            jwt_expires_seconds,
        }
    }

    /// 执行登录逻辑：密码校验 + 生成 JWT + 返回 CurrentUser
    pub fn execute(
        &self,
        user: &User,
        password: &str,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<(String, CurrentUser), AppError> {
        // Argon2 密码校验
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized("用户名或密码错误".to_string()))?;

        if !user.is_active {
            return Err(AppError::PermissionDenied("用户已被禁用".to_string()));
        }

        // 生成 JWT
        let now = Utc::now();
        let exp = now + Duration::seconds(self.jwt_expires_seconds);

        let claims = JwtClaims {
            sub: user.user_id,
            username: user.username.clone(),
            roles: roles.clone(),
            permissions: permissions.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: self.jwt_issuer.clone(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let current_user = CurrentUser {
            user_id: user.user_id,
            username: user.username.clone(),
            full_name: user.full_name.clone(),
            email: user.email.clone(),
            roles,
            permissions,
        };

        info!(user_id = %user.user_id, username = %user.username, "用户登录成功");

        Ok((token, current_user))
    }
}
