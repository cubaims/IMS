use crate::domain::User;
use chrono::{DateTime, Utc};
use cuba_shared::AppError;
use sqlx::{Pool, Postgres};
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

    /// 根据用户名查询用户（登录时使用）
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
        .await?;

        match row {
            Some(r) => Ok(Some(User {
                user_id: r.user_id,
                username: r.username,
                password_hash: r.password_hash,
                full_name: r.full_name,
                email: r.email,
                role_id: r.role_id,
                is_active: r.is_active.unwrap_or(true),
                created_at: r
                    .created_at
                    .map(|t| {
                        DateTime::<Utc>::from_timestamp(t.unix_timestamp(), 0)
                            .unwrap_or_else(|| Utc::now())
                    })
                    .unwrap_or_else(|| Utc::now()),
                updated_at: r
                    .updated_at
                    .map(|t| {
                        DateTime::<Utc>::from_timestamp(t.unix_timestamp(), 0)
                            .unwrap_or_else(|| Utc::now())
                    })
                    .unwrap_or_else(|| Utc::now()),
            })),
            None => Ok(None),
        }
    }

    /// 获取用户的所有角色（主角色 + 多对多角色）
    pub async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        let mut roles = Vec::new();

        // 主角色
        if let Some(primary_role) = sqlx::query_scalar!(
            "SELECT role_id FROM sys.sys_users WHERE user_id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        {
            if let Some(role) = primary_role {
                roles.push(role);
            }
        }

        // 多对多角色
        let additional_roles: Vec<String> = sqlx::query_scalar!(
            r#"
            SELECT r.role_id
            FROM sys.sys_user_roles ur
            JOIN sys.sys_roles r ON r.role_id = ur.role_id
            WHERE ur.user_id = $1
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        roles.extend(additional_roles);
        roles.sort();
        roles.dedup();

        Ok(roles)
    }

    /// 获取用户权限（直接 + 通过角色）
    pub async fn get_user_permissions(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        let permissions: Vec<String> = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT permission_code
            FROM sys.sys_user_permissions
            WHERE (user_id = $1
                   OR role_id IN (SELECT role_id FROM sys.sys_user_roles WHERE user_id = $1))
              AND granted = true
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(permissions)
    }

    /// 写入审计日志
    pub async fn write_audit_log(
        &self,
        user_id: Option<Uuid>,
        action: &str,
        ip_address: Option<String>,
    ) -> Result<(), AppError> {
        // 将 IP 地址字符串解析为 IpNetwork，如果解析失败则使用 None
        let ip_network = ip_address.and_then(|ip| {
            ip.parse::<std::net::IpAddr>().ok().and_then(|addr| {
                use ipnetwork::IpNetwork;
                match addr {
                    std::net::IpAddr::V4(v4) => {
                        IpNetwork::V4(ipnetwork::Ipv4Network::new(v4, 32).ok()?)
                    }
                    std::net::IpAddr::V6(v6) => {
                        IpNetwork::V6(ipnetwork::Ipv6Network::new(v6, 128).ok()?)
                    }
                }
                .into()
            })
        });

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
        .await?;

        info!(?user_id, action = %action, "审计日志已记录");
        Ok(())
    }
}
