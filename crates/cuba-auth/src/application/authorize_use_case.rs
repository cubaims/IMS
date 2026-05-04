use cuba_shared::{AppError, CurrentUser};

pub struct AuthorizeUseCase;

impl AuthorizeUseCase {
    pub fn require_permission(user: &CurrentUser, permission: &str) -> Result<(), AppError> {
        if user.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!("缺少必要权限: {}", permission)))
        }
    }

    pub fn require_any_permission(user: &CurrentUser, permissions: &[&str]) -> Result<(), AppError> {
        if permissions.iter().any(|p| user.has_permission(p)) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!("缺少以下任意权限之一: {:?}", permissions)))
        }
    }

    pub fn require_role(user: &CurrentUser, role: &str) -> Result<(), AppError> {
        if user.has_role(role) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!("缺少必要角色: {}", role)))
        }
    }
}