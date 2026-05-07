/// 全局应用配置。
///
/// 调用约定：在 `main.rs` 启动时调用一次 `dotenvy::dotenv().ok()`,然后
/// 调用 `Settings::from_env()`。本结构体内部**不**再调用 `dotenvy`,
/// 避免重复 IO 与配置职责发散。
#[derive(Clone, Debug)]
pub struct Settings {
    // ============ DB / HTTP ============
    pub database_url: String,
    pub bind_addr: String,
    pub port: u16,

    // ============ JWT ============
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_expires_seconds: i64,

    // ============ DB 连接池 ============
    pub db_max_conn: u32,
    pub db_min_conn: u32,
    pub db_acquire_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub db_max_lifetime_secs: u64,

    // ============ Worker 调度 ============
    /// 物化视图刷新间隔（默认 5）
    pub worker_refresh_interval_minutes: u32,
    /// 低库存检查间隔（默认 10）
    pub worker_low_stock_check_minutes: u32,
    /// MRP 运行间隔（默认 30）
    pub worker_mrp_run_minutes: u32,
    /// 保留多少天的审计日志（默认 90）
    pub worker_audit_cleanup_days: u32,
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://ims:ims@localhost:5432/ims_workspace".to_string()),
            bind_addr: std::env::var("IMS_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            port: parse_env_u16("PORT", 8080),

            jwt_secret: std::env::var("IMS_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            jwt_issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "cuba-ims".to_string()),
            jwt_expires_seconds: parse_env_i64("JWT_EXPIRES_SECONDS", 86_400),

            db_max_conn: parse_env_u32("DB_MAX_CONN", 32),
            db_min_conn: parse_env_u32("DB_MIN_CONN", 4),
            db_acquire_timeout_secs: parse_env_u64("DB_ACQUIRE_TIMEOUT_SECS", 5),
            db_idle_timeout_secs: parse_env_u64("DB_IDLE_TIMEOUT_SECS", 600),
            db_max_lifetime_secs: parse_env_u64("DB_MAX_LIFETIME_SECS", 1_800),

            worker_refresh_interval_minutes: parse_env_u32("WORKER_REFRESH_INTERVAL_MINUTES", 5),
            worker_low_stock_check_minutes: parse_env_u32("WORKER_LOW_STOCK_CHECK_MINUTES", 10),
            worker_mrp_run_minutes: parse_env_u32("WORKER_MRP_RUN_MINUTES", 30),
            worker_audit_cleanup_days: parse_env_u32("WORKER_AUDIT_CLEANUP_DAYS", 90),
        }
    }
}

// ===== helpers: env 解析失败时回退默认值,而不是 panic =====

fn parse_env_u16(key: &str, default: u16) -> u16 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
