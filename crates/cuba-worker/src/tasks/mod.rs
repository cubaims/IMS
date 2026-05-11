use cuba_shared::WorkerSettings;
use sqlx::PgPool;
use tokio::time::{Duration, sleep};
use tracing::info;

pub mod audit_cleanup;
pub mod low_stock_alert;
pub mod materialized_views;
pub mod mrp_suggestion;

/// 启动所有后台任务。
pub async fn start_all_tasks(pool: PgPool, settings: WorkerSettings) {
    info!("🚀 cuba-worker 启动所有后台任务");

    tokio::spawn(materialized_views::refresh_all_materialized_views(
        pool.clone(),
        u64::from(settings.materialized_view_refresh_minutes),
    ));
    tokio::spawn(low_stock_alert::low_stock_alert_task(
        pool.clone(),
        u64::from(settings.low_stock_check_minutes),
    ));
    tokio::spawn(mrp_suggestion::mrp_suggestion_task(
        pool.clone(),
        u64::from(settings.mrp_run_minutes),
    ));
    tokio::spawn(audit_cleanup::audit_log_cleanup_task(
        pool.clone(),
        settings.audit_cleanup_days,
    ));

    // 保持主进程运行
    loop {
        sleep(Duration::from_secs(3600)).await;
    }
}
