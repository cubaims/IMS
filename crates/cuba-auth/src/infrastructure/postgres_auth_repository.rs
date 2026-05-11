use crate::domain::User;
use cuba_shared::{AppError, map_auth_db_error};
use sqlx::{Pool, Postgres, Row};
use time::OffsetDateTime;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresAuthRepository {
    pool: Pool<Postgres>,
}

pub struct StoredRefreshToken {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

impl PostgresAuthRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 根据用户名查询用户(登录时使用)
    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT
                user_id,
                username,
                password_hash,
                full_name,
                email,
                role_id,
                is_active,
                created_at,
                updated_at
            FROM sys.sys_users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        row.map(row_to_user).transpose()
    }

    /// 根据 user_id 查询用户(供 `me` 接口现查全字段使用)
    pub async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT
                user_id,
                username,
                password_hash,
                full_name,
                email,
                role_id,
                is_active,
                created_at,
                updated_at
            FROM sys.sys_users
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        row.map(row_to_user).transpose()
    }

    /// 获取用户的所有角色:`sys_users.role_id`(主角色) + `sys_user_roles`(扩展角色)。
    ///
    /// 用两次查询而不是 UNION,因为 `sys_users.role_id` 可空、
    /// `sys_user_roles.role_id` 不可空,sqlx 编译期对 UNION 列的可空性推断
    /// 不一定靠得住,显式两步更稳。
    pub async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        // 1) 扩展角色(列假设非空,与典型 FK 设计一致)
        let mut roles: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT role_id
            FROM sys.sys_user_roles
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        // 2) 主角色(可空列,可能没有这一行)
        let primary: Option<String> =
            sqlx::query_scalar("SELECT role_id FROM sys.sys_users WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_auth_db_error)?
                .flatten();

        if let Some(role) = primary {
            roles.push(role);
        }

        roles.sort();
        roles.dedup();
        Ok(roles)
    }

    /// 获取用户权限。
    /// 来源:
    /// 1) `sys_user_permissions.user_id = $1`(用户直接授权)
    /// 2) `sys_user_permissions.role_id` 在用户所有角色集合内
    ///    (角色集合 = `sys_users.role_id` ∪ `sys_user_roles`)
    /// 之前的实现遗漏了 `sys_users.role_id` 这条主角色路径。
    pub async fn get_user_permissions(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        let permissions: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT permission_code
            FROM sys.sys_user_permissions
            WHERE (
                user_id = $1
                OR role_id IN (
                    SELECT role_id FROM sys.sys_user_roles WHERE user_id = $1
                    UNION
                    SELECT role_id FROM sys.sys_users
                    WHERE user_id = $1 AND role_id IS NOT NULL
                )
            )
              AND granted = true
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        Ok(permissions)
    }

    /// 写入审计日志。失败时记日志但**不**让登录主流程失败。
    pub async fn write_audit_log(
        &self,
        user_id: Option<Uuid>,
        action: &str,
        ip_address: Option<String>,
    ) -> Result<(), AppError> {
        let ip_network: Option<ipnetwork::IpNetwork> = ip_address
            .as_ref()
            .and_then(|ip| ip.parse::<std::net::IpAddr>().ok())
            .map(ipnetwork::IpNetwork::from);

        sqlx::query(
            r#"
            INSERT INTO sys.sys_audit_log (user_id, action, ip_address, created_at)
            VALUES ($1, $2, $3, NOW())
            "#,
        )
        .bind(user_id)
        .bind(action)
        .bind(ip_network)
        .execute(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        info!(?user_id, action = %action, ip = ?ip_address, "审计日志已记录");
        Ok(())
    }

    pub async fn save_refresh_token(
        &self,
        token_id: Uuid,
        user_id: Uuid,
        selector: &str,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO sys.sys_refresh_tokens
                (token_id, user_id, selector, token_hash, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
        )
        .bind(token_id)
        .bind(user_id)
        .bind(selector)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        Ok(())
    }

    pub async fn find_refresh_token_by_selector(
        &self,
        selector: &str,
    ) -> Result<Option<StoredRefreshToken>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT token_id, user_id, token_hash, expires_at, revoked_at
            FROM sys.sys_refresh_tokens
            WHERE selector = $1
            "#,
        )
        .bind(selector)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_auth_db_error)?;

        let Some(r) = row else {
            return Ok(None);
        };

        Ok(Some(StoredRefreshToken {
            token_id: r.try_get("token_id").map_err(map_auth_db_error)?,
            user_id: r.try_get("user_id").map_err(map_auth_db_error)?,
            token_hash: r.try_get("token_hash").map_err(map_auth_db_error)?,
            expires_at: r.try_get("expires_at").map_err(map_auth_db_error)?,
            revoked_at: r.try_get("revoked_at").map_err(map_auth_db_error)?,
        }))
    }

    pub async fn rotate_refresh_token(
        &self,
        old_token_id: Uuid,
        new_token_id: Uuid,
        user_id: Uuid,
        selector: &str,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(map_auth_db_error)?;

        sqlx::query(
            r#"
            INSERT INTO sys.sys_refresh_tokens
                (token_id, user_id, selector, token_hash, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
        )
        .bind(new_token_id)
        .bind(user_id)
        .bind(selector)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(map_auth_db_error)?;

        let updated = sqlx::query(
            r#"
            UPDATE sys.sys_refresh_tokens
            SET revoked_at = NOW(), replaced_by = $2
            WHERE token_id = $1
              AND user_id = $3
              AND revoked_at IS NULL
              AND expires_at > NOW()
            "#,
        )
        .bind(old_token_id)
        .bind(new_token_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_auth_db_error)?
        .rows_affected();

        if updated != 1 {
            tx.rollback().await.map_err(map_auth_db_error)?;
            return Err(AppError::Unauthorized("REFRESH_TOKEN_INVALID".to_string()));
        }

        tx.commit().await.map_err(map_auth_db_error)?;
        Ok(())
    }
}

fn row_to_user(row: sqlx::postgres::PgRow) -> Result<User, AppError> {
    Ok(User {
        user_id: row.try_get("user_id").map_err(map_auth_db_error)?,
        username: row.try_get("username").map_err(map_auth_db_error)?,
        password_hash: row.try_get("password_hash").map_err(map_auth_db_error)?,
        full_name: row.try_get("full_name").map_err(map_auth_db_error)?,
        email: row.try_get("email").map_err(map_auth_db_error)?,
        role_id: row.try_get("role_id").map_err(map_auth_db_error)?,
        is_active: row
            .try_get::<Option<bool>, _>("is_active")
            .map_err(map_auth_db_error)?
            .unwrap_or(true),
        created_at: row
            .try_get::<Option<OffsetDateTime>, _>("created_at")
            .map_err(map_auth_db_error)?
            .unwrap_or_else(OffsetDateTime::now_utc),
        updated_at: row
            .try_get::<Option<OffsetDateTime>, _>("updated_at")
            .map_err(map_auth_db_error)?
            .unwrap_or_else(OffsetDateTime::now_utc),
    })
}
