use crate::domain::User;
use crate::infrastructure::StoredRefreshToken;
use cuba_shared::{AppError, CurrentUser};
use time::OffsetDateTime;

/// Runtime model for access token invalidation.
///
/// Access tokens are short-lived self-contained JWTs. Request authentication
/// validates the token cryptographically and trusts embedded roles and
/// permissions until expiry. Fresh user status and grants are loaded when a
/// refresh token is exchanged.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AccessTokenInvalidationPolicy {
    /// Do not query the user or grant tables for every authenticated request.
    ShortLivedSelfContained,
}

/// Default access token TTL: 15 minutes.
pub const DEFAULT_ACCESS_TOKEN_EXPIRES_SECONDS: i64 = 900;

/// The current access token invalidation strategy.
pub const ACCESS_TOKEN_INVALIDATION_POLICY: AccessTokenInvalidationPolicy =
    AccessTokenInvalidationPolicy::ShortLivedSelfContained;

pub fn ensure_refresh_token_usable(
    stored: &StoredRefreshToken,
    now: OffsetDateTime,
) -> Result<(), AppError> {
    if stored.revoked_at.is_some() || stored.expires_at <= now {
        return Err(refresh_token_invalid());
    }

    Ok(())
}

pub fn ensure_refresh_user_enabled(user: &User) -> Result<(), AppError> {
    if !user.is_active {
        return Err(AppError::PermissionDenied("用户已被禁用".to_string()));
    }

    Ok(())
}

pub fn current_user_from_fresh_grants(
    user: &User,
    roles: Vec<String>,
    permissions: Vec<String>,
) -> CurrentUser {
    CurrentUser {
        user_id: user.user_id,
        username: user.username.clone(),
        full_name: user.full_name.clone(),
        email: user.email.clone(),
        roles,
        permissions,
    }
}

pub fn refresh_token_invalid() -> AppError {
    AppError::Unauthorized("REFRESH_TOKEN_INVALID".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        LoginUseCase, issue_refresh_token, parse_refresh_token, verify_access_token,
        verify_refresh_secret,
    };
    use crate::domain::User;
    use crate::infrastructure::StoredRefreshToken;
    use argon2::{
        Argon2, PasswordHasher,
        password_hash::{SaltString, rand_core::OsRng},
    };
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    fn user(is_active: bool) -> User {
        User {
            user_id: Uuid::nil(),
            username: "tester".to_string(),
            password_hash: "not-used".to_string(),
            full_name: Some("Tester".to_string()),
            email: Some("tester@example.com".to_string()),
            role_id: None,
            is_active,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn user_with_password(is_active: bool, password: &str) -> User {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("test password hash")
            .to_string();

        User {
            password_hash,
            ..user(is_active)
        }
    }

    fn stored_refresh_token(
        token_id: Uuid,
        user_id: Uuid,
        token_hash: String,
        expires_at: OffsetDateTime,
        revoked_at: Option<OffsetDateTime>,
    ) -> StoredRefreshToken {
        StoredRefreshToken {
            token_id,
            user_id,
            token_hash,
            expires_at,
            revoked_at,
        }
    }

    #[test]
    fn access_token_default_ttl_is_short_lived() {
        assert_eq!(
            ACCESS_TOKEN_INVALIDATION_POLICY,
            AccessTokenInvalidationPolicy::ShortLivedSelfContained
        );
        assert_eq!(DEFAULT_ACCESS_TOKEN_EXPIRES_SECONDS, 900);
    }

    #[test]
    fn disabled_user_cannot_login_or_refresh() {
        let use_case = LoginUseCase::new(
            "test-secret".to_string(),
            "ims-test".to_string(),
            DEFAULT_ACCESS_TOKEN_EXPIRES_SECONDS,
        );
        let login_err = use_case
            .execute(
                &user_with_password(false, "correct-password"),
                "correct-password",
                vec![],
                vec![],
            )
            .expect_err("disabled user rejected during login");
        assert!(matches!(login_err, AppError::PermissionDenied(_)));

        let err = ensure_refresh_user_enabled(&user(false)).expect_err("disabled user rejected");

        assert!(matches!(err, AppError::PermissionDenied(_)));
    }

    #[test]
    fn revoked_or_expired_refresh_token_is_rejected() {
        let now = OffsetDateTime::UNIX_EPOCH + Duration::hours(1);
        let revoked = stored_refresh_token(
            Uuid::new_v4(),
            Uuid::nil(),
            "hash".to_string(),
            now + Duration::minutes(5),
            Some(now),
        );
        let expired =
            stored_refresh_token(Uuid::new_v4(), Uuid::nil(), "hash".to_string(), now, None);

        assert!(matches!(
            ensure_refresh_token_usable(&revoked, now),
            Err(AppError::Unauthorized(_))
        ));
        assert!(matches!(
            ensure_refresh_token_usable(&expired, now),
            Err(AppError::Unauthorized(_))
        ));
    }

    #[test]
    fn permission_revocation_takes_effect_when_refresh_issues_new_access_token() {
        let user = user(true);
        let use_case = LoginUseCase::new(
            "test-secret".to_string(),
            "ims-test".to_string(),
            DEFAULT_ACCESS_TOKEN_EXPIRES_SECONDS,
        );

        let stale_access = use_case
            .issue_access_token(
                &user,
                &["WMS_USER".to_string()],
                &["report:read".to_string()],
            )
            .expect("stale token");
        let refreshed_access = use_case
            .issue_access_token(&user, &["WMS_USER".to_string()], &[])
            .expect("refreshed token");

        let stale_claims =
            verify_access_token(&stale_access, "test-secret", "ims-test").expect("stale claims");
        let refreshed_claims = verify_access_token(&refreshed_access, "test-secret", "ims-test")
            .expect("refreshed claims");

        assert_eq!(stale_claims.permissions, vec!["report:read"]);
        assert!(refreshed_claims.permissions.is_empty());
    }

    #[test]
    fn refresh_token_rotation_makes_previous_refresh_token_unusable() {
        let now = OffsetDateTime::UNIX_EPOCH + Duration::hours(1);
        let old = issue_refresh_token(3_600).expect("old refresh");
        let new = issue_refresh_token(3_600).expect("new refresh");
        let old_parsed = parse_refresh_token(&old.token).expect("old parsed");
        let new_parsed = parse_refresh_token(&new.token).expect("new parsed");
        let old_stored = stored_refresh_token(
            old.token_id,
            Uuid::nil(),
            old.token_hash.clone(),
            now + Duration::hours(1),
            Some(now),
        );
        let new_stored = stored_refresh_token(
            new.token_id,
            Uuid::nil(),
            new.token_hash.clone(),
            now + Duration::hours(1),
            None,
        );

        assert!(matches!(
            ensure_refresh_token_usable(&old_stored, now),
            Err(AppError::Unauthorized(_))
        ));
        ensure_refresh_token_usable(&new_stored, now).expect("replacement token usable");
        verify_refresh_secret(&new_parsed, &new.token_hash).expect("new token valid");

        let err = verify_refresh_secret(&old_parsed, &new.token_hash)
            .expect_err("old secret cannot validate replacement token hash");
        assert!(matches!(err, AppError::Unauthorized(_)));
    }
}
