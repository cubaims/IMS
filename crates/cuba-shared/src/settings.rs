pub struct Settings {
    // ... 已有字段 ...

    // Worker 配置
    pub worker_refresh_interval_minutes: u32,     // 物化视图刷新间隔（默认 5）
    pub worker_low_stock_check_minutes: u32,      // 低库存检查间隔（默认 10）
    pub worker_mrp_run_minutes: u32,              // MRP 运行间隔（默认 30）
    pub worker_audit_cleanup_days: u32,           // 保留多少天的审计日志（默认 90）
}