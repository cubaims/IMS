use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use tracing::info;

pub mod materialized_views;
pub mod low_stock_alert;
pub mod mrp_suggestion;
pub mod audit_cleanup;

/// 启动所有后台任务（临时版：直接传配置参数）
pub async fn start_all_tasks(
    pool: PgPool,
    audit_cleanup_days: u32,
    materialized_refresh_minutes: u64,
    low_stock_check_minutes: u64,
    mrp_run_minutes: u64,
) {
    info!("🚀 cuba-worker 启动所有后台任务");

    tokio::spawn(materialized_views::refresh_all_materialized_views(pool.clone()));
    tokio::spawn(low_stock_alert::low_stock_alert_task(pool.clone()));
    tokio::spawn(mrp_suggestion::mrp_suggestion_task(pool.clone()));
    tokio::spawn(audit_cleanup::audit_log_cleanup_task(
        pool.clone(),
        audit_cleanup_days,
    ));

    // 保持主进程运行
    loop {
        sleep(Duration::from_secs(3600)).await;
    }
}