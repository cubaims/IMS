use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

/// 物化视图刷新任务
pub async fn refresh_all_materialized_views(pool: PgPool) {
    info!("物化视图刷新任务已启动");

    loop {
        let start = std::time::Instant::now();

        match do_refresh(&pool).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                info!("✅ 物化视图刷新完成，耗时 {:?}", elapsed);
            }
            Err(e) => error!("❌ 物化视图刷新失败: {}", e),
        }

        sleep(Duration::from_secs(300)).await; // 5 分钟一次
    }
}

async fn do_refresh(pool: &PgPool) -> anyhow::Result<()> {
    let views = [
        "rpt_current_stock",
        "rpt_inventory_value",
        "rpt_quality_status",
        "rpt_mrp_shortage",
        "rpt_low_stock_alert",
        "rpt_stock_by_zone",
        "rpt_bin_stock_summary",
    ];

    for view in views {
        let sql = format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {}", view);
        if let Err(e) = sqlx::query(&sql).execute(pool).await {
            error!("刷新 {} 失败: {}", view, e);
        } else {
            info!("   ✅ {}", view);
        }
    }

    Ok(())
}