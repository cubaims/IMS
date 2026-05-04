use axum::{Router, routing::get};
use cuba_shared::AppState;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cuba_api=debug,cuba_auth=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 读取环境变量
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("IMS_JWT_SECRET")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .unwrap_or_else(|_| "dev-secret-change-me".to_string());

    // 创建数据库连接池
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let state = AppState {
        db_pool: pool,
        jwt_secret,
    };

    // 路由注册
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", cuba_auth::interface::routes::routes())
        .nest(
            "/api/master-data",
            cuba_master_data::interface::routes::routes(),
        )
        .nest(
            "/api/inventory",
            cuba_inventory::interface::routes::routes(),
        )
        .nest(
            "/api/purchase-orders",
            cuba_purchase::interface::routes::routes(),
        )
        .nest("/api/sales-orders", cuba_sales::interface::routes::routes())
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🚀 cuba-api (Phase 2) listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
