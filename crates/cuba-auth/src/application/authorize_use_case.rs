use cuba_shared::{AppError, CurrentUser};

/// 业务层权限检查用例（推荐在 Handler / Service 中使用）
pub struct AuthorizeUseCase;

impl AuthorizeUseCase {
    pub fn require_permission(user: &CurrentUser, permission: &str) -> Result<(), AppError> {
        if user.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::PermissionDenied(format!(
                "缺少必要权限: {}",
                permission
            )))
        }
    }

    pub fn require_any_permission(
        user: &CurrentUser,
        permissions: &[&str],
    ) -> Result<(), AppError> {
        if user.has_any_permission(permissions) {
            Ok(())
        } else {
            Err(AppError::PermissionDenied(format!(
                "缺少以下任意权限之一: {:?}",
                permissions
            )))
        }
    }

    pub fn require_role(user: &CurrentUser, role: &str) -> Result<(), AppError> {
        if user.has_role(role) {
            Ok(())
        } else {
            Err(AppError::PermissionDenied(format!(
                "缺少必要角色: {}",
                role
            )))
        }
    }
}
