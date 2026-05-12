use crate::application::{
    MrpIdGenerator, MrpMasterRepository, MrpPlannerGateway, MrpRunQuery, MrpRunRepository,
    MrpRunSummary, MrpSuggestionQuery, MrpSuggestionRepository,
};
use crate::domain::{
    MaterialId, MrpError, MrpResult, MrpRun, MrpRunId, MrpRunStatus, MrpSuggestion,
    MrpSuggestionId, MrpSuggestionStatus, MrpSuggestionType, Operator, ProductVariantId,
};
use async_trait::async_trait;
use cuba_shared::{AppError, Page};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Value, json};
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

    pub async fn count_suggestions_for_run(&self, run_id: &MrpRunId) -> MrpResult<(u64, u64)> {
        let suggestion_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_mrp_suggestions
            WHERE run_id = $1
            "#,
        )
        .bind(run_id.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        let shortage_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_mrp_suggestions
            WHERE run_id = $1
              AND COALESCE(shortage_qty, 0) > 0
            "#,
        )
        .bind(run_id.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        Ok((suggestion_count.max(0) as u64, shortage_count.max(0) as u64))
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

fn map_mrp_db_error(error: sqlx::Error) -> MrpError {
    match cuba_shared::map_mrp_db_error(error) {
        AppError::Business {
            code: "MRP_RUN_NOT_FOUND",
            ..
        } => MrpError::MrpRunNotFound,
        AppError::Business {
            code: "MRP_SUGGESTION_NOT_FOUND",
            ..
        } => MrpError::MrpSuggestionNotFound,
        AppError::Business {
            code: "MRP_SUGGESTION_STATUS_INVALID",
            ..
        } => MrpError::MrpSuggestionStatusInvalid,
        AppError::Business {
            code: "MRP_MATERIAL_NOT_FOUND_OR_INACTIVE",
            ..
        } => MrpError::MaterialNotFoundOrInactive,
        AppError::Business {
            code: "MRP_VARIANT_NOT_FOUND",
            ..
        } => MrpError::ProductVariantNotFound,
        AppError::Business {
            code: "MRP_RUN_FAILED",
            ..
        } => MrpError::MrpRunFailed,
        AppError::Business { code, message } => {
            MrpError::BusinessRuleViolation(format!("{code}: {message}"))
        }
        AppError::Validation(message) => MrpError::BusinessRuleViolation(message),
        AppError::Database { .. } | AppError::Internal(_) => MrpError::MrpRunFailed,
        other => MrpError::BusinessRuleViolation(other.public_message()),
    }
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
        MrpRunStatus::Cancelled => "取消",
    }
}

fn run_status_from_db(value: &str) -> MrpRunStatus {
    match value {
        "运行中" => MrpRunStatus::Running,
        "完成" => MrpRunStatus::Completed,
        "取消" => MrpRunStatus::Cancelled,
        _ => MrpRunStatus::Running,
    }
}

fn suggestion_type_from_db(value: Option<String>) -> MrpSuggestionType {
    match value.as_deref() {
        Some("生产订单") => MrpSuggestionType::Production,
        Some("采购申请") => MrpSuggestionType::Purchase,
        Some("转储建议") | Some("调拨建议") => MrpSuggestionType::Transfer,
        _ => MrpSuggestionType::Purchase,
    }
}

fn suggestion_type_to_db(value: MrpSuggestionType) -> &'static str {
    match value {
        MrpSuggestionType::Purchase => "采购申请",
        MrpSuggestionType::Production => "生产订单",
        MrpSuggestionType::Transfer => "转储建议",
    }
}

fn suggestion_status_to_code(value: MrpSuggestionStatus) -> &'static str {
    match value {
        MrpSuggestionStatus::Open => "OPEN",
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
        Some("NEW") | Some("OPEN") | None => MrpSuggestionStatus::Open,
        _ => MrpSuggestionStatus::Open,
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
        "cancelled_by": suggestion.cancelled_by.as_ref().map(|x| x.as_str().to_string()),
        "cancelled_at": suggestion.cancelled_at,
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
    let base_material_id = row
        .try_get::<Option<String>, _>("base_material_id")
        .ok()
        .flatten();

    let demand_date: Option<Date> = row.get("demand_date");
    let demand_date = demand_date
        .map(date_to_offset_datetime)
        .unwrap_or_else(OffsetDateTime::now_utc);

    Ok(MrpRun {
        id: MrpRunId::new(run_id),
        material_id: base_material_id.map(MaterialId::new),
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

    let status_code = remarks_meta
        .get("status")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| row.try_get::<Option<String>, _>("status").ok().flatten());

    let status = suggestion_status_from_code(status_code.as_deref());

    let confirmed_by = remarks_meta
        .get("confirmed_by")
        .and_then(|v| v.as_str())
        .map(Operator::new)
        .or_else(|| {
            row.try_get::<Option<String>, _>("confirmed_by")
                .ok()
                .flatten()
                .map(Operator::new)
        });

    let confirmed_at = remarks_meta
        .get("confirmed_at")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .or_else(|| row.try_get::<Option<OffsetDateTime>, _>("confirmed_at").ok().flatten());

    let cancelled_by = remarks_meta
        .get("cancelled_by")
        .and_then(|v| v.as_str())
        .map(Operator::new)
        .or_else(|| {
            row.try_get::<Option<String>, _>("cancelled_by")
                .ok()
                .flatten()
                .map(Operator::new)
        });

    let cancelled_at = remarks_meta
        .get("cancelled_at")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .or_else(|| row.try_get::<Option<OffsetDateTime>, _>("cancelled_at").ok().flatten());

    let remark = row
        .try_get::<Option<String>, _>("cancelled_reason")
        .ok()
        .flatten()
        .or_else(|| remark_from_meta(&remarks_meta));

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
        bom_level: row.get::<Option<i32>, _>("bom_level").unwrap_or(0),
        gross_requirement_qty: Decimal::from(
            row.get::<Option<i32>, _>("gross_requirement_qty")
                .unwrap_or(0),
        ),
        required_qty: Decimal::from(row.get::<Option<i32>, _>("required_qty").unwrap_or(0)),
        available_qty: Decimal::from(row.get::<Option<i32>, _>("available_qty").unwrap_or(0)),
        safety_stock_qty: Decimal::from(row.get::<Option<i32>, _>("safety_stock_qty").unwrap_or(0)),
        shortage_qty: Decimal::from(row.get::<Option<i32>, _>("shortage_qty").unwrap_or(0)),
        net_requirement_qty: Decimal::from(row.get::<Option<i32>, _>("shortage_qty").unwrap_or(0)),
        suggested_qty: Decimal::from(
            row.get::<Option<i32>, _>("suggested_order_qty")
                .unwrap_or(0),
        ),
        recommended_bin: row.get::<Option<String>, _>("recommended_bin"),
        recommended_batch: row.get::<Option<String>, _>("recommended_batch"),
        lead_time_days: row.get::<Option<i32>, _>("lead_time_days"),
        priority: row.get::<Option<i32>, _>("priority"),
        required_date,
        suggested_date,
        supplier_id: None,
        work_center_id: None,
        status,
        created_at,
        confirmed_by,
        confirmed_at,
        cancelled_by,
        cancelled_at,
        remark,
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
        .bind(
            run.demand_qty.to_i32().ok_or_else(|| {
                MrpError::BusinessRuleViolation("需求数量超过 i32 范围".to_string())
            })?,
        )
        .bind(offset_datetime_to_date(run.demand_date))
        .bind(30_i32)
        .bind(run_status_to_db(run.status))
        .bind(run.created_by.as_str())
        .bind(run.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        Ok(run.id.clone())
    }

    async fn find_by_id(&self, run_id: &MrpRunId) -> MrpResult<Option<MrpRun>> {
        let row = sqlx::query(
            r#"
            SELECT
                r.run_id,
                r.run_date,
                r.variant_code,
                pv.base_material_id,
                r.demand_qty,
                r.demand_date,
                r.planning_horizon,
                r.status,
                r.created_by,
                r.created_at
            FROM wms.wms_mrp_runs r
            LEFT JOIN mdm.mdm_product_variants pv ON pv.variant_code = r.variant_code
            WHERE r.run_id = $1
            "#,
        )
        .bind(run_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        match row {
            Some(row) => Ok(Some(mrp_run_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn lock_by_id(&self, run_id: &MrpRunId) -> MrpResult<MrpRun> {
        let row = sqlx::query(
            r#"
            SELECT
                r.run_id,
                r.run_date,
                r.variant_code,
                pv.base_material_id,
                r.demand_qty,
                r.demand_date,
                r.planning_horizon,
                r.status,
                r.created_by,
                r.created_at
            FROM wms.wms_mrp_runs r
            LEFT JOIN mdm.mdm_product_variants pv ON pv.variant_code = r.variant_code
            WHERE r.run_id = $1
            FOR UPDATE
            "#,
        )
        .bind(run_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

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
        .map_err(map_mrp_db_error)?;

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
        let material_id = query.material_id.as_ref().map(|x| x.as_str().to_string());

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_mrp_runs r
            LEFT JOIN mdm.mdm_product_variants pv ON pv.variant_code = r.variant_code
            WHERE ($1::text IS NULL OR r.status = $1)
              AND ($2::text IS NULL OR r.variant_code = $2)
              AND ($3::text IS NULL OR pv.base_material_id = $3)
              AND ($4::timestamptz IS NULL OR r.created_at >= $4)
              AND ($5::timestamptz IS NULL OR r.created_at < $5)
            "#,
        )
        .bind(status)
        .bind(variant_code.clone())
        .bind(material_id.clone())
        .bind(query.date_from)
        .bind(query.date_to)
        .fetch_one(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                r.run_id,
                r.run_date,
                r.variant_code,
                pv.base_material_id,
                r.demand_qty,
                r.demand_date,
                r.planning_horizon,
                r.status,
                r.created_by,
                r.created_at
            FROM wms.wms_mrp_runs r
            LEFT JOIN mdm.mdm_product_variants pv ON pv.variant_code = r.variant_code
            WHERE ($1::text IS NULL OR r.status = $1)
              AND ($2::text IS NULL OR r.variant_code = $2)
              AND ($3::text IS NULL OR pv.base_material_id = $3)
              AND ($4::timestamptz IS NULL OR r.created_at >= $4)
              AND ($5::timestamptz IS NULL OR r.created_at < $5)
            ORDER BY r.created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(status)
        .bind(variant_code)
        .bind(material_id)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

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
        let id = suggestion_id
            .as_str()
            .parse::<i64>()
            .map_err(|_| MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string()))?;

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
                status,
                confirmed_by,
                confirmed_at,
                cancelled_by,
                cancelled_at,
                cancelled_reason,
                remarks,
                created_at
            FROM wms.wms_mrp_suggestions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        match row {
            Some(row) => Ok(Some(mrp_suggestion_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn lock_by_id(&self, suggestion_id: &MrpSuggestionId) -> MrpResult<MrpSuggestion> {
        let id = suggestion_id
            .as_str()
            .parse::<i64>()
            .map_err(|_| MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string()))?;

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
                status,
                confirmed_by,
                confirmed_at,
                cancelled_by,
                cancelled_at,
                cancelled_reason,
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
        .map_err(map_mrp_db_error)?;

        let Some(row) = row else {
            return Err(MrpError::MrpSuggestionNotFound);
        };

        mrp_suggestion_from_row(&row)
    }

    async fn update(&self, suggestion: &MrpSuggestion) -> MrpResult<()> {
        let id =
            suggestion.id.as_str().parse::<i64>().map_err(|_| {
                MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string())
            })?;

        let remarks = suggestion_remarks_to_json(suggestion);

        let result =
            sqlx::query(
                r#"
            UPDATE wms.wms_mrp_suggestions
            SET
                suggested_order_type = $2,
                suggested_order_qty = $3,
                remarks = $4,
                status = $5,
                confirmed_by = $6,
                confirmed_at = $7,
                cancelled_by = $8,
                cancelled_at = $9,
                cancelled_reason = $10
            WHERE id = $1
            "#,
            )
            .bind(id)
            .bind(suggestion_type_to_db(suggestion.suggestion_type))
            .bind(suggestion.suggested_qty.to_i32().ok_or_else(|| {
                MrpError::BusinessRuleViolation("建议数量超过 i32 范围".to_string())
            })?)
            .bind(remarks)
            .bind(suggestion_status_to_code(suggestion.status))
            .bind(
                suggestion
                    .confirmed_by
                    .as_ref()
                    .map(|operator| operator.as_str().to_string()),
            )
            .bind(suggestion.confirmed_at)
            .bind(
                suggestion
                    .cancelled_by
                    .as_ref()
                    .map(|operator| operator.as_str().to_string()),
            )
            .bind(suggestion.cancelled_at)
            .bind(suggestion.remark.clone())
            .execute(&self.pool)
            .await
            .map_err(map_mrp_db_error)?;

        if result.rows_affected() == 0 {
            return Err(MrpError::MrpSuggestionNotFound);
        }

        Ok(())
    }

    async fn confirm(
        &self,
        suggestion_id: &MrpSuggestionId,
        confirmed_by: Operator,
        remark: Option<String>,
        now: OffsetDateTime,
    ) -> MrpResult<MrpSuggestion> {
        let mut tx = self.pool.begin().await.map_err(map_mrp_db_error)?;
        let mut suggestion = lock_suggestion_in_tx(&mut tx, suggestion_id).await?;

        suggestion.confirm(confirmed_by, now)?;
        suggestion.remark = remark;

        update_suggestion_in_tx(&mut tx, &suggestion).await?;
        tx.commit().await.map_err(map_mrp_db_error)?;

        Ok(suggestion)
    }

    async fn cancel(
        &self,
        suggestion_id: &MrpSuggestionId,
        cancelled_by: Operator,
        reason: String,
        now: OffsetDateTime,
    ) -> MrpResult<MrpSuggestion> {
        let mut tx = self.pool.begin().await.map_err(map_mrp_db_error)?;
        let mut suggestion = lock_suggestion_in_tx(&mut tx, suggestion_id).await?;

        suggestion.cancel(cancelled_by, now, reason)?;

        update_suggestion_in_tx(&mut tx, &suggestion).await?;
        tx.commit().await.map_err(map_mrp_db_error)?;

        Ok(suggestion)
    }

    async fn list(&self, query: MrpSuggestionQuery) -> MrpResult<Page<MrpSuggestion>> {
        let page = query.page.page.max(1);
        let page_size = query.page.page_size.clamp(1, 200);
        let offset = ((page - 1).saturating_mul(page_size)) as usize;
        let limit = page_size as usize;

        let run_id = query.run_id.as_ref().map(|x| x.as_str().to_string());
        let material_id = query.material_id.as_ref().map(|x| x.as_str().to_string());
        let suggestion_type = query.suggestion_type.map(suggestion_type_to_db);

        // v9 表中没有独立 status 字段，应用层状态暂存在 remarks JSON 中。
        // 因此这里先在 SQL 层筛 run_id / material_id / suggestion_type，
        // 再在 Rust 层筛 status / required_date，最后做分页。
        //
        // 这样可以保证 total 与 items 一致。
        // 后续如果数据库增加 status 字段，可以把 status/date 过滤下推到 SQL。
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
                status,
                confirmed_by,
                confirmed_at,
                cancelled_by,
                cancelled_at,
                cancelled_reason,
                remarks,
                created_at
            FROM wms.wms_mrp_suggestions
            WHERE ($1::text IS NULL OR run_id = $1)
              AND ($2::text IS NULL OR material_id = $2)
              AND ($3::text IS NULL OR suggested_order_type = $3)
            ORDER BY priority ASC, id ASC
            "#,
        )
        .bind(run_id)
        .bind(material_id)
        .bind(suggestion_type)
        .fetch_all(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        let mut filtered = Vec::with_capacity(rows.len());

        for row in rows {
            let suggestion = mrp_suggestion_from_row(&row)?;

            if let Some(status) = query.status {
                if suggestion.status != status {
                    continue;
                }
            }

            if let Some(date_from) = query.required_date_from {
                if suggestion.required_date < date_from {
                    continue;
                }
            }

            if let Some(date_to) = query.required_date_to {
                if suggestion.required_date >= date_to {
                    continue;
                }
            }

            if query.only_shortage.unwrap_or(false) && suggestion.shortage_qty <= Decimal::ZERO {
                continue;
            }

            filtered.push(suggestion);
        }

        let total = filtered.len() as u64;

        let items = filtered.into_iter().skip(offset).take(limit).collect();

        Ok(Page::new(items, total, page, page_size))
    }
}

async fn lock_suggestion_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    suggestion_id: &MrpSuggestionId,
) -> MrpResult<MrpSuggestion> {
    let id = suggestion_id
        .as_str()
        .parse::<i64>()
        .map_err(|_| MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string()))?;

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
            status,
            confirmed_by,
            confirmed_at,
            cancelled_by,
            cancelled_at,
            cancelled_reason,
            remarks,
            created_at
        FROM wms.wms_mrp_suggestions
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_mrp_db_error)?;

    let Some(row) = row else {
        return Err(MrpError::MrpSuggestionNotFound);
    };

    mrp_suggestion_from_row(&row)
}

async fn update_suggestion_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    suggestion: &MrpSuggestion,
) -> MrpResult<()> {
    let id = suggestion
        .id
        .as_str()
        .parse::<i64>()
        .map_err(|_| MrpError::BusinessRuleViolation("MRP 建议 ID 必须是数字".to_string()))?;

    let remarks = suggestion_remarks_to_json(suggestion);

    let result = sqlx::query(
        r#"
        UPDATE wms.wms_mrp_suggestions
        SET
            suggested_order_type = $2,
            suggested_order_qty = $3,
            remarks = $4,
            status = $5,
            confirmed_by = $6,
            confirmed_at = $7,
            cancelled_by = $8,
            cancelled_at = $9,
            cancelled_reason = $10
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(suggestion_type_to_db(suggestion.suggestion_type))
    .bind(
        suggestion
            .suggested_qty
            .to_i32()
            .ok_or_else(|| MrpError::BusinessRuleViolation("建议数量超过 i32 范围".to_string()))?,
    )
    .bind(remarks)
    .bind(suggestion_status_to_code(suggestion.status))
    .bind(
        suggestion
            .confirmed_by
            .as_ref()
            .map(|operator| operator.as_str().to_string()),
    )
    .bind(suggestion.confirmed_at)
    .bind(
        suggestion
            .cancelled_by
            .as_ref()
            .map(|operator| operator.as_str().to_string()),
    )
    .bind(suggestion.cancelled_at)
    .bind(suggestion.remark.clone())
    .execute(&mut **tx)
    .await
    .map_err(map_mrp_db_error)?;

    if result.rows_affected() == 0 {
        return Err(MrpError::MrpSuggestionNotFound);
    }

    Ok(())
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
    async fn run_mrp_function(&self, run: &MrpRun) -> MrpResult<MrpRunId> {
        let variant_code = run
            .product_variant_id
            .as_ref()
            .ok_or(MrpError::ProductVariantRequired)?;

        let demand_qty = run
            .demand_qty
            .to_i32()
            .ok_or_else(|| MrpError::BusinessRuleViolation("需求数量超过 i32 范围".to_string()))?;

        let demand_date = offset_datetime_to_date(run.demand_date);

        let db_run_id: String = sqlx::query_scalar(
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
        .map_err(map_mrp_db_error)?;

        Ok(MrpRunId::new(db_run_id))
    }
}

// =============================================================================
// MrpMasterRepository
// =============================================================================

#[async_trait]
impl MrpMasterRepository for PostgresMrpStore {
    async fn material_exists_and_active(&self, material_id: &MaterialId) -> MrpResult<bool> {
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
        .map_err(map_mrp_db_error)?;

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
        .map_err(map_mrp_db_error)?;

        Ok(exists)
    }

    async fn find_active_variant_by_material(
        &self,
        material_id: &MaterialId,
    ) -> MrpResult<Option<ProductVariantId>> {
        let variant_code: Option<String> = sqlx::query_scalar(
            r#"
            SELECT variant_code
            FROM mdm.mdm_product_variants
            WHERE base_material_id = $1
              AND is_active = TRUE
            ORDER BY
                CASE WHEN bom_id IS NULL THEN 1 ELSE 0 END,
                variant_code ASC
            LIMIT 1
            "#,
        )
        .bind(material_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_mrp_db_error)?;

        Ok(variant_code.map(ProductVariantId::new))
    }
}
