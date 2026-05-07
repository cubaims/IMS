use anyhow::Result;
use cuba_worker::tasks;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    // ====================== Worker 配置（临时硬编码） ======================
    let db_url = std::env::var("DATABASE_URL")
        .expect("请设置 DATABASE_URL 环境变量");

    let audit_cleanup_days: u32 = std::env::var("WORKER_AUDIT_CLEANUP_DAYS")
        .unwrap_or_else(|_| "90".to_string())
        .parse()
        .unwrap_or(90);

    let materialized_refresh_minutes: u64 = std::env::var("WORKER_MATERIALIZED_VIEW_REFRESH_MINUTES")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5) * 60;

    let low_stock_check_minutes: u64 = std::env::var("WORKER_LOW_STOCK_CHECK_MINUTES")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10) * 60;

    let mrp_run_minutes: u64 = std::env::var("WORKER_MRP_RUN_MINUTES")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .unwrap_or(30) * 60;

    // ====================== 数据库连接池 ======================
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_url)
        .await?;

    info!("✅ cuba-worker 启动成功 | 审计日志保留 {} 天", audit_cleanup_days);

    // 启动所有任务（传入需要的参数）
    let worker_handle = tokio::spawn(tasks::start_all_tasks(
        pool,
        audit_cleanup_days,
        materialized_refresh_minutes,
        low_stock_check_minutes,
        mrp_run_minutes,
    ));

    // 优雅关闭
    tokio::select! {
        _ = worker_handle => info!("所有 worker 任务已结束"),
        _ = shutdown_signal() => info!("收到关闭信号，worker 正在优雅退出..."),
    }

    info!("👋 cuba-worker 已完全关闭");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("cuba_worker=info")),
        )
        .init();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("failed to listen for Ctrl+C");
}