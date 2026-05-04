use cuba_shared::Settings;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::from_env();
    tracing_subscriber::fmt().init();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&settings.database_url)
        .await?;
    sqlx::query("SELECT rpt.refresh_all_materialized_views()")
        .execute(&pool)
        .await?;
    tracing::info!("materialized views refreshed");
    Ok(())
}
