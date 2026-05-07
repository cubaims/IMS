use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

/// MRP 建议生成任务（临时版：只接受 pool）
pub async fn mrp_suggestion_task(pool: PgPool) {
    info!("MRP 建议生成任务已启动");

    loop {
        match run_mrp_suggestion(&pool).await {
            Ok(_) => info!("✅ MRP 建议生成完成"),
            Err(e) => error!("❌ MRP 建议生成失败: {}", e),
        }

        // 默认 30 分钟执行一次（后续可改成配置）
        sleep(Duration::from_secs(30 * 60)).await;
    }
}

async fn run_mrp_suggestion(pool: &PgPool) -> anyhow::Result<()> {
    info!("正在执行 MRP 建议生成...");

    // 调用数据库中的 MRP 函数
    sqlx::query("SELECT wms.fn_run_mrp()")
        .execute(pool)
        .await?;

    info!("MRP 建议生成完成");

    Ok(())
}