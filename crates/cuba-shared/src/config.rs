#[derive(Clone, Debug)]
pub struct Settings {
    pub database_url: String,
    pub bind_addr: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_expires_seconds: i64,

    // 新增：数据库连接池配置
    pub db_max_conn: u32,
    pub db_min_conn: u32,
    pub db_acquire_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub db_max_lifetime_secs: u64,

    // 新增：端口（从 bind_addr 解析，或单独配置）
    pub port: u16,
}

impl Settings {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://ims:ims@localhost:5432/ims_workspace".to_string()),
            bind_addr: std::env::var("IMS_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            jwt_secret: std::env::var("IMS_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            jwt_issuer: std::env::var("JWT_ISSUER")
                .unwrap_or_else(|_| "cuba-ims".to_string()),
            jwt_expires_seconds: std::env::var("JWT_EXPIRES_SECONDS")
                .map(|v| v.parse().unwrap_or(86400))
                .unwrap_or(86400),

            db_max_conn: std::env::var("DB_MAX_CONN")
                .map(|v| v.parse().unwrap_or(32))
                .unwrap_or(32),
            db_min_conn: std::env::var("DB_MIN_CONN")
                .map(|v| v.parse().unwrap_or(4))
                .unwrap_or(4),
            db_acquire_timeout_secs: std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
                .map(|v| v.parse().unwrap_or(5))
                .unwrap_or(5),
            db_idle_timeout_secs: std::env::var("DB_IDLE_TIMEOUT_SECS")
                .map(|v| v.parse().unwrap_or(600))
                .unwrap_or(600),
            db_max_lifetime_secs: std::env::var("DB_MAX_LIFETIME_SECS")
                .map(|v| v.parse().unwrap_or(1800))
                .unwrap_or(1800),

            port: std::env::var("PORT")
                .map(|v| v.parse().unwrap_or(8080))
                .unwrap_or(8080),
        }
    }
}