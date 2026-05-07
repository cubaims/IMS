use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

/// 审计日志清理任务
///
/// 每天执行一次，删除过期的审计日志，保持 sys_audit_log 表大小可控
pub async fn audit_log_cleanup_task(pool: PgPool, retain_days: u32) {
    info!("审计日志清理任务已启动（保留 {} 天）", retain_days);

    loop {
        match cleanup_old_logs(&pool, retain_days).await {
            Ok(deleted) => {
                if deleted > 0 {
                    info!("✅ 已清理 {} 条过期审计日志", deleted);
                }
            }
            Err(e) => error!("❌ 审计日志清理失败: {}", e),
        }

        // 每天执行一次
        sleep(Duration::from_secs(24 * 3600)).await;
    }
}

async fn cleanup_old_logs(pool: &PgPool, retain_days: u32) -> anyhow::Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM sys.sys_audit_log
        WHERE created_at < NOW() - INTERVAL '1 day' * $1
        "#,
        retain_days as i32
    )
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}