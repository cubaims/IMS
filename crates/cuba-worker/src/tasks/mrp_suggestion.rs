use cuba_shared::map_worker_db_error;
use sqlx::PgPool;
use tokio::time::{Duration, sleep};
use tracing::{error, info};

/// MRP 建议生成任务（临时版：只接受 pool）
pub async fn mrp_suggestion_task(pool: PgPool, interval_minutes: u64) {
    let interval_seconds = interval_minutes.max(1) * 60;
    info!(
        "MRP 建议生成任务已启动（每 {} 分钟执行一次）",
        interval_minutes.max(1)
    );

    loop {
        match run_mrp_suggestion(&pool).await {
            Ok(_) => info!("✅ MRP 建议生成完成"),
            Err(e) => error!("❌ MRP 建议生成失败: {}", e),
        }

        sleep(Duration::from_secs(interval_seconds)).await;
    }
}

async fn run_mrp_suggestion(pool: &PgPool) -> anyhow::Result<()> {
    info!("正在执行 MRP 建议生成...");

    // 调用数据库中的 MRP 函数
    sqlx::query("SELECT wms.fn_run_mrp()")
        .execute(pool)
        .await
        .map_err(|err| anyhow::anyhow!(map_worker_db_error(err)))?;

    info!("MRP 建议生成完成");

    Ok(())
}
