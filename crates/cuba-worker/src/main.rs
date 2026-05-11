use anyhow::Result;
use cuba_shared::{Settings, map_worker_db_error};
use cuba_worker::tasks;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let settings = Settings::from_env()?;
    let worker_settings = settings.worker.clone();

    // ====================== 数据库连接池 ======================
    let pool = PgPoolOptions::new()
        .max_connections(settings.db_max_conn)
        .min_connections(settings.db_min_conn)
        .acquire_timeout(Duration::from_secs(settings.db_acquire_timeout_secs))
        .idle_timeout(Some(Duration::from_secs(settings.db_idle_timeout_secs)))
        .max_lifetime(Some(Duration::from_secs(settings.db_max_lifetime_secs)))
        .test_before_acquire(true)
        .connect(&settings.database_url)
        .await
        .map_err(|err| anyhow::anyhow!(map_worker_db_error(err)))?;

    info!(
        "✅ cuba-worker 启动成功 | 审计日志保留 {} 天 | 物化视图刷新 {} 分钟 | 低库存检查 {} 分钟 | MRP {} 分钟",
        worker_settings.audit_cleanup_days,
        worker_settings.materialized_view_refresh_minutes,
        worker_settings.low_stock_check_minutes,
        worker_settings.mrp_run_minutes
    );

    // 启动所有任务（传入需要的参数）
    let worker_handle = tokio::spawn(tasks::start_all_tasks(pool, worker_settings));

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
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");
}
