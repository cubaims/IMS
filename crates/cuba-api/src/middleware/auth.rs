//! JWT 认证中间件。
//!
//! 解析 Authorization Bearer token，将 cuba_shared::CurrentUser 注入 request extensions。
//! 该中间件不按请求查库确认用户状态或权限版本。当前认证模型是短期自包含
//! access token:这里校验签名、issuer、exp、token_type 后信任 claims 内的
//! roles/permissions,禁用用户和权限撤销在 refresh/login 重新查库时生效。

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use cuba_auth::{VerifyError, verify_access_token};
use cuba_shared::{AppError, AppState, CurrentUser};

const BEARER: &str = "Bearer ";

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer(&request)?;

    let claims = verify_access_token(token, &state.jwt_secret, &state.jwt_issuer)
        .map_err(map_verify_error)?;

    let current_user = CurrentUser {
        user_id: claims.sub,
        username: claims.username,
        // JWT 不携带这两个字段;handler 需要时应自行查 DB(见 cuba-auth me handler)。
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

    // Bearer 大小写不敏感(RFC 6750)
    let prefix_len = BEARER.len();
    if header.len() < prefix_len || !header[..prefix_len].eq_ignore_ascii_case(BEARER) {
        return Err(AppError::Unauthorized("TOKEN_INVALID".to_string()));
    }

    Ok(header[prefix_len..].trim())
}

fn map_verify_error(error: VerifyError) -> AppError {
    match error {
        VerifyError::Expired => AppError::Unauthorized("TOKEN_EXPIRED".to_string()),
        VerifyError::Invalid(_) | VerifyError::BadIssuer | VerifyError::BadSignature => {
            AppError::Unauthorized("TOKEN_INVALID".to_string())
        }
    }
}
