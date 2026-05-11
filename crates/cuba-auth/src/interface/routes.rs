use axum::{
    Router,
    routing::{get, post},
};
use cuba_shared::AppState;

use super::handlers;

/// 公开路由(不需要登录)
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
}

/// 需要认证的路由
pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(handlers::me))
        // 当前登录用户的授权视图；系统角色管理使用 /api/system/roles。
        .route("/roles", get(handlers::roles))
        .route("/permissions", get(handlers::permissions))
}

pub fn routes() -> Router<AppState> {
    public_routes().merge(protected_routes())
}
