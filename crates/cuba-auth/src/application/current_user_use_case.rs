use crate::domain::User;
use cuba_shared::CurrentUser;

pub struct GetCurrentUserUseCase;

impl GetCurrentUserUseCase {
    pub fn execute(
        user: User,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> CurrentUser {
        CurrentUser {
            user_id: user.user_id,
            username: user.username,
            full_name: user.full_name,
            email: user.email,
            roles,
            permissions,
        }
    }
}