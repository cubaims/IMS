//! JWT 认证中间件。
//!
//! 解析 Authorization Bearer token，将 cuba_shared::CurrentUser 注入 request extensions。

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use cuba_auth::{verify_access_token, VerifyError};
use cuba_shared::{AppError, AppState, CurrentUser};

const BEARER: &str = "Bearer ";

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer(&request)?;

    let claims = verify_access_token(token, &state.jwt_secret).map_err(map_verify_error)?;

    let current_user = CurrentUser {
        user_id: claims.sub,
        username: claims.username,
        full_name: None,
        email: None,
        roles: claims.roles,
        permissions: claims.permissions,
    };

    request.extensions_mut().insert(current_user);

    Ok(next.run(request).await)
}

fn extract_bearer(req: &Request) -> Result<&str, AppError> {
    let header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("UNAUTHORIZED".to_string()))?
        .to_str()
        .map_err(|_| AppError::Unauthorized("TOKEN_INVALID".to_string()))?;

    if !header.starts_with(BEARER) {
        return Err(AppError::Unauthorized("TOKEN_INVALID".to_string()));
    }

    Ok(header[BEARER.len()..].trim())
}

fn map_verify_error(error: VerifyError) -> AppError {
    match error {
        VerifyError::Expired => AppError::Unauthorized("TOKEN_EXPIRED".to_string()),
        VerifyError::Invalid(_) | VerifyError::BadIssuer | VerifyError::BadSignature => {
            AppError::Unauthorized("TOKEN_INVALID".to_string())
        }
    }
}
