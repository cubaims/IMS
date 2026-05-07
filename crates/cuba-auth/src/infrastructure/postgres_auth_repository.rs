use crate::domain::User;
use cuba_shared::AppError;
use sqlx::{Pool, Postgres};
use time::OffsetDateTime;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresAuthRepository {
    pool: Pool<Postgres>,
}

impl PostgresAuthRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 根据用户名查询用户(登录时使用)
    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query!(
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
            username
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::raw_database)?;

        Ok(row.map(|r| User {
            user_id: r.user_id,
            username: r.username,
            password_hash: r.password_hash,
            full_name: r.full_name,
            email: r.email,
            role_id: r.role_id,
            is_active: r.is_active.unwrap_or(true),
            created_at: r.created_at.unwrap_or_else(OffsetDateTime::now_utc),
            updated_at: r.updated_at.unwrap_or_else(OffsetDateTime::now_utc),
        }))
    }

    /// 根据 user_id 查询用户(供 `me` 接口现查全字段使用)
    pub async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError> {
        let row = sqlx::query!(
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
            user_id
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::raw_database)?;

        Ok(row.map(|r| User {
            user_id: r.user_id,
            username: r.username,
            password_hash: r.password_hash,
            full_name: r.full_name,
            email: r.email,
            role_id: r.role_id,
            is_active: r.is_active.unwrap_or(true),
            created_at: r.created_at.unwrap_or_else(OffsetDateTime::now_utc),
            updated_at: r.updated_at.unwrap_or_else(OffsetDateTime::now_utc),
        }))
    }

    /// 获取用户的所有角色:`sys_users.role_id`(主角色) + `sys_user_roles`(扩展角色)。
    ///
    /// 用两次查询而不是 UNION,因为 `sys_users.role_id` 可空、
    /// `sys_user_roles.role_id` 不可空,sqlx 编译期对 UNION 列的可空性推断
    /// 不一定靠得住,显式两步更稳。
    pub async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        // 1) 扩展角色(列假设非空,与典型 FK 设计一致)
        let mut roles: Vec<String> = sqlx::query_scalar!(
            r#"
            SELECT role_id
            FROM sys.sys_user_roles
            WHERE user_id = $1
            "#,
            user_id
        )
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::raw_database)?;

        // 2) 主角色(可空列,可能没有这一行)
        let primary = sqlx::query_scalar!(
            "SELECT role_id FROM sys.sys_users WHERE user_id = $1",
            user_id
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::raw_database)?
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
        let permissions: Vec<String> = sqlx::query_scalar!(
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
            user_id
        )
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::raw_database)?;

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

        sqlx::query!(
            r#"
            INSERT INTO sys.sys_audit_log (user_id, action, ip_address, created_at)
            VALUES ($1, $2, $3, NOW())
            "#,
            user_id,
            action,
            ip_network as Option<ipnetwork::IpNetwork>
        )
            .execute(&self.pool)
            .await
            .map_err(AppError::raw_database)?;

        info!(?user_id, action = %action, ip = ?ip_address, "审计日志已记录");
        Ok(())
    }
}
