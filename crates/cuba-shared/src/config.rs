use std::{env, error::Error, fmt};

/// 全局应用配置。
///
/// 调用约定：在 `main.rs` 启动时调用一次 `dotenvy::dotenv().ok()`,然后
/// 调用 `Settings::from_env()`。本结构体内部**不**再调用 `dotenvy`,
/// 避免重复 IO 与配置职责发散。
#[derive(Clone, Debug)]
pub struct Settings {
    // ============ DB / HTTP ============
    pub database_url: String,
    /// Final HTTP listen address. `IMS_BIND_ADDR` is a full `host:port` value
    /// and takes precedence; `PORT` is only used when `IMS_BIND_ADDR` is unset.
    pub bind_addr: String,
    /// Whether the API process should apply SQLx migrations before serving.
    pub run_migrations: bool,

    // ============ JWT ============
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_expires_seconds: i64,
    pub jwt_refresh_expires_seconds: i64,

    // ============ DB 连接池 ============
    pub db_max_conn: u32,
    pub db_min_conn: u32,
    pub db_acquire_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub db_max_lifetime_secs: u64,

    // ============ Worker 调度 ============
    /// Worker 调度配置。
    pub worker: WorkerSettings,
}

/// Worker 进程共享调度配置。
#[derive(Clone, Debug)]
pub struct WorkerSettings {
    /// 物化视图刷新间隔（分钟，默认 5）。
    pub materialized_view_refresh_minutes: u32,
    /// 低库存检查间隔（分钟，默认 10）。
    pub low_stock_check_minutes: u32,
    /// MRP 运行间隔（分钟，默认 30）。
    pub mrp_run_minutes: u32,
    /// 保留多少天的审计日志（默认 90）。
    pub audit_cleanup_days: u32,
}

/// 配置加载错误。
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConfigError {
    /// 必需环境变量未设置或为空。
    MissingEnv { key: &'static str },
    /// 环境变量已设置，但值无法解析。
    InvalidEnv {
        key: &'static str,
        value: String,
        expected: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnv { key } => write!(
                f,
                "missing required environment variable {key} (set it in .env or the process environment)"
            ),
            Self::InvalidEnv {
                key,
                value,
                expected,
            } => write!(
                f,
                "invalid value for environment variable {key}: {value:?} (expected {expected})"
            ),
        }
    }
}

impl Error for ConfigError {}

impl Settings {
    pub fn from_env() -> Result<Self, ConfigError> {
        let db_max_conn = parse_env_u32("DB_MAX_CONN", 32)?;
        let db_min_conn = parse_env_u32("DB_MIN_CONN", 4)?;
        validate_db_pool_bounds(db_min_conn, db_max_conn)?;

        Ok(Self {
            database_url: required_env("DATABASE_URL")?,
            bind_addr: resolve_bind_addr(env::var("IMS_BIND_ADDR").ok())?,
            run_migrations: parse_env_bool("RUN_MIGRATIONS", false)?,

            jwt_secret: env_string("IMS_JWT_SECRET", "change-me-in-production"),
            jwt_issuer: env_string("JWT_ISSUER", "cuba-ims"),
            jwt_expires_seconds: parse_env_i64("JWT_EXPIRES_SECONDS", 900)?,
            jwt_refresh_expires_seconds: parse_env_i64("JWT_REFRESH_EXPIRES_SECONDS", 2_592_000)?,

            db_max_conn,
            db_min_conn,
            db_acquire_timeout_secs: parse_env_u64("DB_ACQUIRE_TIMEOUT_SECS", 5)?,
            db_idle_timeout_secs: parse_env_u64("DB_IDLE_TIMEOUT_SECS", 600)?,
            db_max_lifetime_secs: parse_env_u64("DB_MAX_LIFETIME_SECS", 1_800)?,

            worker: WorkerSettings::from_env()?,
        })
    }
}

impl WorkerSettings {
    /// 从环境变量加载 Worker 调度配置。
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            materialized_view_refresh_minutes: parse_env_u32(
                "WORKER_MATERIALIZED_VIEW_REFRESH_MINUTES",
                5,
            )?,
            low_stock_check_minutes: parse_env_u32("WORKER_LOW_STOCK_CHECK_MINUTES", 10)?,
            mrp_run_minutes: parse_env_u32("WORKER_MRP_RUN_MINUTES", 30)?,
            audit_cleanup_days: parse_env_u32("WORKER_AUDIT_CLEANUP_DAYS", 90)?,
        })
    }
}

// ===== helpers: env 解析失败时返回清晰错误 =====

fn required_env(key: &'static str) -> Result<String, ConfigError> {
    let value = env::var(key).map_err(|_| ConfigError::MissingEnv { key })?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::MissingEnv { key });
    }
    Ok(trimmed.to_owned())
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_string(key: &str, default: &str) -> String {
    optional_env(key).unwrap_or_else(|| default.to_owned())
}

fn parse_env_u16(key: &'static str, default: u16) -> Result<u16, ConfigError> {
    parse_optional_env(key, default, "an unsigned 16-bit integer")
}

fn parse_env_bool(key: &'static str, default: bool) -> Result<bool, ConfigError> {
    let Some(value) = optional_env(key) else {
        return Ok(default);
    };

    parse_bool(&value).ok_or(ConfigError::InvalidEnv {
        key,
        value,
        expected: "one of true/false, 1/0, yes/no, on/off",
    })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn resolve_bind_addr(bind_addr: Option<String>) -> Result<String, ConfigError> {
    if let Some(value) = bind_addr {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }

    Ok(format!("0.0.0.0:{}", parse_env_u16("PORT", 8080)?))
}

fn parse_env_u32(key: &'static str, default: u32) -> Result<u32, ConfigError> {
    parse_optional_env(key, default, "an unsigned 32-bit integer")
}

fn parse_env_u64(key: &'static str, default: u64) -> Result<u64, ConfigError> {
    parse_optional_env(key, default, "an unsigned 64-bit integer")
}

fn parse_env_i64(key: &'static str, default: i64) -> Result<i64, ConfigError> {
    parse_optional_env(key, default, "a signed 64-bit integer")
}

fn parse_optional_env<T>(
    key: &'static str,
    default: T,
    expected: &'static str,
) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    let Some(value) = optional_env(key) else {
        return Ok(default);
    };

    value.parse().map_err(|_| ConfigError::InvalidEnv {
        key,
        value,
        expected,
    })
}

fn validate_db_pool_bounds(db_min_conn: u32, db_max_conn: u32) -> Result<(), ConfigError> {
    if db_min_conn <= db_max_conn {
        return Ok(());
    }

    Err(ConfigError::InvalidEnv {
        key: "DB_MIN_CONN",
        value: db_min_conn.to_string(),
        expected: "an integer less than or equal to DB_MAX_CONN",
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_bool, resolve_bind_addr};

    #[test]
    fn bind_addr_prefers_ims_bind_addr() {
        assert_eq!(
            resolve_bind_addr(Some(String::from("127.0.0.1:18080"))).as_deref(),
            Ok("127.0.0.1:18080")
        );
    }

    #[test]
    fn bind_addr_trims_configured_value() {
        assert_eq!(
            resolve_bind_addr(Some(String::from("  127.0.0.1:18080  "))).as_deref(),
            Ok("127.0.0.1:18080")
        );
    }

    #[test]
    fn bool_parser_accepts_common_env_values() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("maybe"), None);
    }
}
