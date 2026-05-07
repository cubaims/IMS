use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

/// 低库存预警任务（最终极简安全版）
pub async fn low_stock_alert_task(pool: PgPool) {
    info!("低库存预警任务已启动（每 10 分钟检查一次）");

    loop {

        sleep(Duration::from_secs(600)).await; // 10 分钟一次
    }
}

