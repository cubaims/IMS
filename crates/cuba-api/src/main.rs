use cuba_shared::{AppState, Settings};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::from_env();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&settings.database_url)
        .await?;

    let app = cuba_api::build_router(AppState { pool });
    let listener = TcpListener::bind(&settings.bind_addr).await?;
    tracing::info!(addr = %settings.bind_addr, "IMS Workspace API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
