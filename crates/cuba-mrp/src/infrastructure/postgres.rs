use crate::application::{
    MrpIdGenerator, MrpMasterRepository, MrpPlannerGateway, MrpRunQuery,
    MrpRunRepository, MrpRunSummary, MrpSuggestionQuery, MrpSuggestionRepository,
};
use crate::domain::{
    MaterialId, MrpError, MrpResult, MrpRun, MrpRunId, MrpRunStatus,
    MrpSuggestion, MrpSuggestionId, MrpSuggestionStatus, MrpSuggestionType,
    Operator, ProductVariantId,
};
use async_trait::async_trait;
use cuba_shared::Page;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

/// PostgreSQL MRP Store。
///
/// 这个对象持有 PgPool。
/// 后续 MRP 的 Repository / Gateway 都由它实现。
#[derive(Clone)]
pub struct PostgresMrpStore {
    pool: PgPool,
}

impl PostgresMrpStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

// =============================================================================
// ID 生成器
// =============================================================================

/// PostgreSQL 版 MRP ID 生成器。
///
/// v9 的 run_id 是 VARCHAR(30)。
/// 这里用 MRP- + UUID 截断，保证长度不超过 30。
#[derive(Debug, Clone, Default)]
pub struct PostgresMrpIdGenerator;

impl PostgresMrpIdGenerator {
    fn next_prefixed_id(prefix: &str) -> String {
        let raw = Uuid::new_v4().simple().to_string();

        // prefix + "-" + 26 = 30，例如 MRP-xxxxxxxxxxxxxxxxxxxxxxxxxx
        let max_random_len = 30usize.saturating_sub(prefix.len() + 1);
        let short = &raw[..max_random_len];

        format!("{prefix}-{short}")
    }
}

impl MrpIdGenerator for PostgresMrpIdGenerator {
    fn next_mrp_run_id(&self) -> MrpRunId {
        MrpRunId::new(Self::next_prefixed_id("MRP"))
    }
}

// =============================================================================
// 通用错误映射
// =============================================================================

fn map_sqlx_error(error: sqlx::Error) -> MrpError {
    MrpError::BusinessRuleViolation(format!("数据库错误：{error}"))
}

// =============================================================================
// 时间转换
// =============================================================================

/// 把 Date 转成 UTC 零点 OffsetDateTime。
fn date_to_offset_datetime(date: Date) -> OffsetDateTime {
    date.with_time(Time::MIDNIGHT).assume_utc()
}

/// 把 OffsetDateTime 转成 Date。
///
/// v9 表里的 demand_date 是 DATE，
/// 所以这里会丢弃时间部分。
fn offset_datetime_to_date(value: OffsetDateTime) -> Date {
    value.date()
}

// =============================================================================
// MRP 状态映射
// =============================================================================

fn run_status_to_db(status: MrpRunStatus) -> &'static str {
    match status {
        MrpRunStatus::Created => "运行中",
        MrpRunStatus::Running => "运行中",
        MrpRunStatus::Completed => "完成",
        MrpRunStatus::Failed => "取消",
    }
}

fn run_status_from_db(value: &str) -> MrpRunStatus {
    match value {
        "运行中" => MrpRunStatus::Running,
        "完成" => MrpRunStatus::Completed,
        "取消" => MrpRunStatus::Failed,
        _ => MrpRunStatus::Running,
    }
}

fn suggestion_type_from_db(value: Option<String>) -> MrpSuggestionType {
    match value.as_deref() {
        Some("生产订单") => MrpSuggestionType::Production,
        Some("采购申请") => MrpSuggestionType::Purchase,
        Some("转储建议") => MrpSuggestionType::Purchase,
        _ => MrpSuggestionType::Purchase,
    }
}

fn suggestion_type_to_db(value: MrpSuggestionType) -> &'static str {
    match value {
        MrpSuggestionType::Purchase => "采购申请",
        MrpSuggestionType::Production => "生产订单",
    }
}

fn suggestion_status_to_code(value: MrpSuggestionStatus) -> &'static str {
    match value {
        MrpSuggestionStatus::New => "NEW",
        MrpSuggestionStatus::Confirmed => "CONFIRMED",
        MrpSuggestionStatus::Cancelled => "CANCELLED",
        MrpSuggestionStatus::Converted => "CONVERTED",
    }
}

fn suggestion_status_from_code(value: Option<&str>) -> MrpSuggestionStatus {
    match value {
        Some("CONFIRMED") => MrpSuggestionStatus::Confirmed,
        Some("CANCELLED") => MrpSuggestionStatus::Cancelled,
        Some("CONVERTED") => MrpSuggestionStatus::Converted,
        _ => MrpSuggestionStatus::New,
    }
}

// =============================================================================
// remarks JSON 映射
// =============================================================================

/// MRP 建议表没有 status 字段，
/// 所以应用层状态写入 remarks JSON。
fn suggestion_remarks_to_json(suggestion: &MrpSuggestion) -> String {
    json!({
        "status": suggestion_status_to_code(suggestion.status),
        "confirmed_by": suggestion.confirmed_by.as_ref().map(|x| x.as_str().to_string()),
        "confirmed_at": suggestion.confirmed_at,
        "remark": suggestion.remark
    })
        .to_string()
}

/// 兼容历史纯文本 remarks。
fn parse_remarks(value: Option<String>) -> Value {
    let Some(text) = value else {
        return json!({});
    };

    serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
        json!({
            "remark": text
        })
    })
}

/// 把 remarks JSON 里的 remark 字段取出来。
fn remark_from_meta(meta: &Value) -> Option<String> {
    meta.get("remark")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// =============================================================================
// Row -> Domain
// =============================================================================

fn mrp_run_from_row(row: &sqlx::postgres::PgRow) -> MrpResult<MrpRun> {
    let run_id: String = row.get("run_id");
    let status_text: String = row.get("status");

    let demand_date: Option<Date> = row.get("demand_date");
    let demand_date = demand_date
        .map(date_to_offset_datetime)
        .unwrap_or_else(OffsetDateTime::now_utc);

    Ok(MrpRun {
        id: MrpRunId::new(run_id),
        material_id: None,
        product_variant_id: row
            .get::<Option<String>, _>("variant_code")
            .map(ProductVariantId::new),
        demand_qty: Decimal::from(row.get::<i32, _>("demand_qty")),
        demand_date,
        status: run_status_from_db(&status_text),
        created_by: Operator::new(
            row.get::<Option<String>, _>("created_by")
                .unwrap_or_else(|| "SYSTEM".to_string()),
        ),
        created_at: row.get::<OffsetDateTime, _>("created_at"),
        started_at: Some(row.get::<OffsetDateTime, _>("run_date")),
        completed_at: None,
        error_message: None,
        remark: None,
    })
}

fn mrp_suggestion_from_row(row: &sqlx::postgres::PgRow) -> MrpResult<MrpSuggestion> {
    let remarks_meta = parse_remarks(row.get::<Option<String>, _>("remarks"));

    let status = suggestion_status_from_code(
        remarks_meta.get("status").and_then(|v| v.as_str()),
    );

    let confirmed_by = remarks_meta
        .get("confirmed_by")
        .and_then(|v| v.as_str())
        .map(Operator::new);

    let confirmed_at = remarks_meta
        .get("confirmed_at")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let created_at: OffsetDateTime = row.get("created_at");

    // v9 建议表没有 required_date / suggested_date，
    // 这里 MVP 先用 created_at 兜底。
    let required_date = created_at;
    let suggested_date = created_at;

    Ok(MrpSuggestion {
        id: MrpSuggestionId::new(row.get::<i64, _>("id").to_string()),
        run_id: MrpRunId::new(row.get::<String, _>("run_id")),
        suggestion_type: suggestion_type_from_db(
            row.get::<Option<String>, _>("suggested_order_type"),
        ),
        material_id: MaterialId::new(row.get::<String, _>("material_id")),
        suggested_qty: Decimal::from(
            row.get::<Option<i32>, _>("suggested_order_qty")
                .unwrap_or(0),
        ),
        required_date,
        suggested_date,
        supplier_id: None,
        work_center_id: None,
        status,
        created_at,
        confirmed_by,
        confirmed_at,
        remark: remark_from_meta(&remarks_meta),
    })
}

// =============================================================================
// MrpRunRepository
// =============================================================================

#[async_trait]
impl MrpRunRepository for PostgresMrpStore {
    async fn create(&self, run: &MrpRun) -> MrpResult<MrpRunId> {
        let variant_code = run
            .product_variant_id
            .as_ref()
            .map(|x| x.as_str().to_string());

        sqlx::query(
            r#"
            INSERT INTO wms.wms_mrp_runs (
                run_id,
                run_date,
                variant_code,
                demand_qty,
                demand_date,
                planning_horizon,
                status,
                created_by,
                created_at
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9
            )
            "#,
        )
            .bind(run.id.as_str())
            .bind(run.started_at.unwrap_or(run.created_at))
            .bind(variant_code)
            .bind(run.demand_qty.to_i32().ok_or_else(|| {
                MrpError::BusinessRuleViolation("需求数量超过 i32 范围".to_string())
            })?)
            .bind(offset_datetime_to_date(run.demand_date))
            .bind(30_i32)
            .bind(run_status_to_db(run.status))
            .bind(run.created_by.as_str())
            .bind(run.created_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(run.id.clone())
    }

    async fn find_by_id(&self, run_id: &MrpRunId) -> MrpResult<Option<MrpRun>> {
        let row = sqlx::query(
            r#"
            SELECT
                run_id,
                run_date,
                variant_code,
                demand_qty,
                demand_date,
                planning_horizon,
                status,
                created_by,
                created_at
            FROM wms.wms_mrp_runs
            WHERE run_id = $1
            "#,
        )
            .bind(run_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            Some(row) => Ok(Some(mrp_run_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn lock_by_id(&self, run_id: &MrpRunId) -> MrpResult<MrpRun> {
        let row = sqlx::query(
            r#"
            SELECT
                run_id,
                run_date,
                variant_code,
                demand_qty,
                demand_date,
                planning_horizon,
                status,
                created_by,
                created_at
            FROM wms.wms_mrp_runs
            WHERE run_id = $1
            FOR UPDATE
            "#,
        )
            .bind(run_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Err(MrpError::MrpRunNotFound);
        };

        mrp_run_from_row(&row)
    }

    async fn update(&self, run: &MrpRun) -> MrpResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_mrp_runs
            SET
                status = $2,
                run_date = COALESCE($3, run_date)
            WHERE run_id = $1
            "#,
        )
            .bind(run.id.as_str())
            .bind(run_status_to_db(run.status))
            .bind(run.started_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(MrpError::MrpRunNotFound);
        }

        Ok(())
    }

    async fn list(&self, query: MrpRunQuery) -> MrpResult<Page<MrpRunSummary>> {
        let page = query.page.page.max(1);
        let page_size = query.page.page_size.clamp(1, 200);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let status = query.status.map(run_status_to_db);
        let variant_code = query
            .product_variant_id
            .as_ref()
            .map(|x| x.as_str().to_string());

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_mrp_runs
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR variant_code = $2)
              AND ($3::timestamptz IS NULL OR created_at >= $3)
              AND ($4::timestamptz IS NULL OR created_at < $4)
            "#,
        )
            .bind(status)
            .bind(variant_code.clone())
            .bind(query.date_from)
            .bind(query.date_to)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                run_id,
                run_date,
                variant_code,
                demand_qty,
                demand_date,
                planning_horizon,
                status,
                created_by,
                created_at
            FROM wms.wms_mrp_runs
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR variant_code = $2)
              AND ($3::timestamptz IS NULL OR created_at >= $3)
              AND ($4::timestamptz IS NULL OR created_at < $4)
            ORDER BY created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
            .bind(status)
            .bind(variant_code)
            .bind(query.date_from)
            .bind(query.date_to)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(rows.len());

        for row in rows {
            let run = mrp_run_from_row(&row)?;

            items.push(MrpRunSummary {
                id: run.id,
                material_id: run.material_id,
                product_variant_id: run.product_variant_id,
                demand_qty: run.demand_qty,
                demand_date: run.demand_date,
                status: run.status,
                created_at: run.created_at,
            });
        }

        Ok(Page::new(items, total as u64, page, page_size))
    }
}

// =============================================================================
// MrpSuggestionRepository
// =============================================================================

#[async_trait]
impl MrpSuggestionRepository for PostgresMrpStore {
    async fn find_by_id(
        &self,
        suggestion_id: &MrpSuggestionId,
    ) -> MrpResult<Option<MrpSuggestion>> {
        let id = suggestion_id.as_str().parse::<i64>().map_err(|_| {
            MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string())
        })?;

        let row = sqlx::query(
            r#"
            SELECT
                id,
                run_id,
                material_id,
                bom_level,
                gross_requirement_qty,
                required_qty,
                available_qty,
                safety_stock_qty,
                shortage_qty,
                suggested_order_type,
                suggested_order_qty,
                recommended_bin,
                recommended_batch,
                lead_time_days,
                priority,
                remarks,
                created_at
            FROM wms.wms_mrp_suggestions
            WHERE id = $1
            "#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            Some(row) => Ok(Some(mrp_suggestion_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn lock_by_id(
        &self,
        suggestion_id: &MrpSuggestionId,
    ) -> MrpResult<MrpSuggestion> {
        let id = suggestion_id.as_str().parse::<i64>().map_err(|_| {
            MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string())
        })?;

        let row = sqlx::query(
            r#"
            SELECT
                id,
                run_id,
                material_id,
                bom_level,
                gross_requirement_qty,
                required_qty,
                available_qty,
                safety_stock_qty,
                shortage_qty,
                suggested_order_type,
                suggested_order_qty,
                recommended_bin,
                recommended_batch,
                lead_time_days,
                priority,
                remarks,
                created_at
            FROM wms.wms_mrp_suggestions
            WHERE id = $1
            FOR UPDATE
            "#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Err(MrpError::MrpSuggestionNotFound);
        };

        mrp_suggestion_from_row(&row)
    }

    async fn update(&self, suggestion: &MrpSuggestion) -> MrpResult<()> {
        let id = suggestion.id.as_str().parse::<i64>().map_err(|_| {
            MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string())
        })?;

        let remarks = suggestion_remarks_to_json(suggestion);

        let result = sqlx::query(
            r#"
            UPDATE wms.wms_mrp_suggestions
            SET
                suggested_order_type = $2,
                suggested_order_qty = $3,
                remarks = $4
            WHERE id = $1
            "#,
        )
            .bind(id)
            .bind(suggestion_type_to_db(suggestion.suggestion_type))
            .bind(suggestion.suggested_qty.to_i32().ok_or_else(|| {
                MrpError::BusinessRuleViolation("建议数量超过 i32 范围".to_string())
            })?)
            .bind(remarks)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(MrpError::MrpSuggestionNotFound);
        }

        Ok(())
    }

    async fn list(
        &self,
        query: MrpSuggestionQuery,
    ) -> MrpResult<Page<MrpSuggestion>> {
        let page = query.page.page.max(1);
        let page_size = query.page.page_size.clamp(1, 200);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let run_id = query.run_id.as_ref().map(|x| x.as_str().to_string());
        let material_id = query.material_id.as_ref().map(|x| x.as_str().to_string());
        let suggestion_type = query.suggestion_type.map(suggestion_type_to_db);

        // v9 表中没有 status 字段，status 只在 remarks JSON 中。
        // 为了避免 SQL 复杂化，MVP 先在数据库层筛 run_id/type/material，
        // status 在 Rust 里二次过滤。
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_mrp_suggestions
            WHERE ($1::text IS NULL OR run_id = $1)
              AND ($2::text IS NULL OR material_id = $2)
              AND ($3::text IS NULL OR suggested_order_type = $3)
            "#,
        )
            .bind(run_id.clone())
            .bind(material_id.clone())
            .bind(suggestion_type)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                id,
                run_id,
                material_id,
                bom_level,
                gross_requirement_qty,
                required_qty,
                available_qty,
                safety_stock_qty,
                shortage_qty,
                suggested_order_type,
                suggested_order_qty,
                recommended_bin,
                recommended_batch,
                lead_time_days,
                priority,
                remarks,
                created_at
            FROM wms.wms_mrp_suggestions
            WHERE ($1::text IS NULL OR run_id = $1)
              AND ($2::text IS NULL OR material_id = $2)
              AND ($3::text IS NULL OR suggested_order_type = $3)
            ORDER BY priority ASC, id ASC
            LIMIT $4 OFFSET $5
            "#,
        )
            .bind(run_id)
            .bind(material_id)
            .bind(suggestion_type)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(rows.len());

        for row in rows {
            let suggestion = mrp_suggestion_from_row(&row)?;

            if let Some(status) = query.status {
                if suggestion.status != status {
                    continue;
                }
            }

            items.push(suggestion);
        }

        Ok(Page::new(items, total as u64, page, page_size))
    }
}

// =============================================================================
// MrpPlannerGateway
// =============================================================================

#[async_trait]
impl MrpPlannerGateway for PostgresMrpStore {
    /// 调用 v9 数据库函数 wms.fn_run_mrp。
    ///
    /// 注意：
    /// v9 函数会自己创建 wms_mrp_runs 和 wms_mrp_suggestions。
    /// 所以严格来说，前面 RunMrpUseCase 里 create(run) 会和函数内部 insert 重复。
    ///
    /// 为了不改数据库函数，MVP 推荐后面把 RunMrpUseCase 调整为：
    /// - 不先 create run
    /// - 直接调用 fn_run_mrp()
    /// - 用返回的 run_id 查询结果
    ///
    /// 当前这个 Gateway 先给出函数调用实现。
    async fn run_mrp_function(&self, run: &MrpRun) -> MrpResult<()> {
        let variant_code = run
            .product_variant_id
            .as_ref()
            .ok_or(MrpError::ProductVariantRequired)?;

        let demand_qty = run.demand_qty.to_i32().ok_or_else(|| {
            MrpError::BusinessRuleViolation("需求数量超过 i32 范围".to_string())
        })?;

        let demand_date = offset_datetime_to_date(run.demand_date);

        let _db_run_id: String = sqlx::query_scalar(
            r#"
            SELECT wms.fn_run_mrp($1, $2, $3, $4, $5)
            "#,
        )
            .bind(variant_code.as_str())
            .bind(demand_qty)
            .bind(demand_date)
            .bind(30_i32)
            .bind(run.created_by.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
    }
}

// =============================================================================
// MrpMasterRepository
// =============================================================================

#[async_trait]
impl MrpMasterRepository for PostgresMrpStore {
    async fn material_exists_and_active(
        &self,
        material_id: &MaterialId,
    ) -> MrpResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM mdm.mdm_materials
                WHERE material_id = $1
                  AND status = '正常'
            )
            "#,
        )
            .bind(material_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(exists)
    }

    async fn product_variant_exists(
        &self,
        product_variant_id: &ProductVariantId,
    ) -> MrpResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM mdm.mdm_product_variants
                WHERE variant_code = $1
                  AND is_active = TRUE
            )
            "#,
        )
            .bind(product_variant_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(exists)
    }
}