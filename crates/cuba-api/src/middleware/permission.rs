use axum::{
    extract::Extension,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use cuba_auth::CurrentUser;

/// 要求拥有指定权限的中间件
pub async fn require_permission<B>(
    permission: &'static str,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let current_user = request
        .extensions()
        .get::<CurrentUser>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if current_user.has_permission(permission) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}