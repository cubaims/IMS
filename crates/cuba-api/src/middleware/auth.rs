use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

use cuba_auth::{JwtClaims, CurrentUser};
use crate::AppState;

/// JWT 认证中间件
/// 解析成功后将 CurrentUser 放入 request.extensions
pub async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // 获取 Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // 解析 JWT
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let claims = token_data.claims;

    // 构造 CurrentUser
    let current_user = CurrentUser {
        user_id: claims.sub,
        username: claims.username,
        full_name: None,
        email: None,
        roles: claims.roles,
        permissions: claims.permissions,
    };

    // 将 CurrentUser 放入 extensions
    request.extensions_mut().insert(current_user);

    // 继续执行后续 handler
    Ok(next.run(request).await)
}