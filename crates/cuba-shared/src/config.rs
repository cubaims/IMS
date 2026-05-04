#[derive(Clone, Debug)]
pub struct Settings {
    pub database_url: String,
    pub bind_addr: String,
    pub jwt_secret: String,
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
        }
    }
}
