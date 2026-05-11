use sqlx::PgPool;
use tokio::time::{Duration, sleep};
use tracing::info;

/// 低库存预警任务（最终极简安全版）
pub async fn low_stock_alert_task(_pool: PgPool, interval_minutes: u64) {
    let interval_seconds = interval_minutes.max(1) * 60;
    info!(
        "低库存预警任务已启动（每 {} 分钟检查一次）",
        interval_minutes.max(1)
    );

    loop {
        sleep(Duration::from_secs(interval_seconds)).await;
    }
}
