use cuba_shared::{AppError, map_reporting_db_error};
use sqlx::PgPool;
use tokio::time::{Duration, sleep};
use tracing::{error, info};

/// 物化视图刷新任务
pub async fn refresh_all_materialized_views(pool: PgPool, interval_minutes: u64) {
    let interval_seconds = interval_minutes.max(1) * 60;
    info!(
        "物化视图刷新任务已启动（每 {} 分钟刷新一次）",
        interval_minutes.max(1)
    );

    loop {
        let start = std::time::Instant::now();

        match do_refresh(&pool).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                info!("✅ 物化视图刷新完成，耗时 {:?}", elapsed);
            }
            Err(e) => error!("❌ 物化视图刷新失败: {}", e),
        }

        sleep(Duration::from_secs(interval_seconds)).await;
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
        if let Err(err) = sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|err| map_materialized_view_db_error(view, err))
        {
            error!("刷新 {} 失败: {}", view, err);
        } else {
            info!("   ✅ {}", view);
        }
    }

    Ok(())
}

fn map_materialized_view_db_error(view: &str, error: sqlx::Error) -> AppError {
    match map_reporting_db_error(error) {
        AppError::Business {
            code: "REPORT_QUERY_FAILED",
            ..
        } => AppError::business(
            "MATERIALIZED_VIEW_REFRESH_FAILED",
            format!("刷新物化视图失败: {view}"),
        ),
        mapped => mapped,
    }
}
