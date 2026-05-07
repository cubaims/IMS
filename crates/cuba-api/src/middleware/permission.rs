//! 权限/角色守卫，放在 auth_middleware 之后使用。

use axum::{extract::Request, middleware::Next, response::Response};
use cuba_shared::{AppError, CurrentUser}; // ← 关键修改

/// 检查权限的通用函数
async fn check<F>(req: Request, next: Next, predicate: F) -> Result<Response, AppError>
where
    F: Fn(&CurrentUser) -> bool,
{
    let user = req
        .extensions()
        .get::<CurrentUser>()
        .ok_or_else(|| AppError::Unauthorized("UNAUTHORIZED".to_string()))?;

    if predicate(user) {
        Ok(next.run(req).await)
    } else {
        Err(AppError::PermissionDenied("PERMISSION_DENIED".to_string()))
    }
}

/// 创建权限检查中间件
///
/// 用法：
/// ```ignore
/// use axum::middleware::from_fn;
///
/// .route("/refresh", post(refresh)
///     .route_layer(from_fn(|req, next| {
///         require_permission("report:refresh", req, next)
///     })))
/// ```
pub async fn require_permission(
    perm: &'static str,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // ← 改成 AppError
    check(req, next, |u| u.has_permission(perm)).await
}

/// 要求至少拥有列表中的一个权限
pub async fn require_any_permission(
    perms: &'static [&'static str],
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // ← 改成 AppError
    check(req, next, |u| u.has_any_permission(perms)).await
}

/// 要求拥有指定角色
pub async fn require_role(
    role: &'static str,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // ← 改成 AppError
    check(req, next, |u| u.has_role(role)).await
}
