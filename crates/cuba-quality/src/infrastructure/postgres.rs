use crate::application::{
    BatchHistoryQuery, BatchQualityRepository, BatchQualityStatusView,
    DefectCodeSnapshot, InspectionCharSnapshot, QualityIdGenerator,
    QualityMasterRepository,
};
use crate::domain::{
    BatchNumber, BatchQuality, BatchQualityAction, BatchQualityHistory,
    BatchQualityStatus, DefectCode, InspectionCharId, InspectionLotId,
    InspectionResultId, MaterialId, Operator, QualityError,
    QualityNotificationId, QualityResult,
};

use crate::application::{
    InspectionLotQuery, InspectionLotRepository, InspectionLotSummary,
    InspectionResultRepository, QualityNotificationQuery,
    QualityNotificationRepository, QualityNotificationSummary,
};
use crate::domain::{
    CreateInspectionLot, CreateInspectionResult, CreateQualityNotification,
    InspectionDecision, InspectionLot, InspectionLotStatus, InspectionLotType,
    InspectionResult, InspectionResultStatus, QualityNotification,
    QualityNotificationSeverity, QualityNotificationStatus,
};
use serde_json::{json, Value};
use async_trait::async_trait;
use cuba_shared::Page;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL 质量模块基础设施对象。
///
/// 这个对象持有 PgPool。
/// 后续所有 Postgres repository 都可以 clone 它使用。
#[derive(Clone)]
pub struct PostgresQualityStore {
    pool: PgPool,
}

impl PostgresQualityStore {
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

/// PostgreSQL 版 ID 生成器。
///
/// 当前先用 UUID 截断生成短 ID，保证能放进 VARCHAR(30)。
/// 生产版后面可以替换成数据库序列或日期流水号。
#[derive(Debug, Clone, Default)]
pub struct PostgresQualityIdGenerator;

impl PostgresQualityIdGenerator {
    fn next_prefixed_id(prefix: &str) -> String {
        let raw = Uuid::new_v4().simple().to_string();

        // 表字段是 VARCHAR(30)。
        // prefix + "-" + 27 位 = 30 位。
        let short = &raw[..27];
        format!("{prefix}-{short}")
    }
}

impl QualityIdGenerator for PostgresQualityIdGenerator {
    fn next_inspection_lot_id(&self) -> InspectionLotId {
        InspectionLotId::new(Self::next_prefixed_id("IL"))
    }

    fn next_inspection_result_id(&self) -> InspectionResultId {
        InspectionResultId::new(Self::next_prefixed_id("IR"))
    }

    fn next_quality_notification_id(&self) -> QualityNotificationId {
        QualityNotificationId::new(Self::next_prefixed_id("QN"))
    }
}

// =============================================================================
// 通用映射函数
// =============================================================================

/// 把 sqlx 错误转换成质量模块业务错误。
///
/// 当前 MVP 先统一包装成 BusinessRuleViolation。
/// 后续可以根据数据库错误码细分为：
/// - 外键不存在
/// - 唯一键冲突
/// - CHECK 约束失败
fn map_sqlx_error(error: sqlx::Error) -> QualityError {
    QualityError::BusinessRuleViolation(format!("数据库错误：{error}"))
}

/// 把领域质量状态转为数据库枚举文本。
///
/// 数据库类型是 mdm.quality_status。
fn batch_status_to_db(status: BatchQualityStatus) -> &'static str {
    match status {
        BatchQualityStatus::PendingInspection => "待检",
        BatchQualityStatus::Qualified => "合格",
        BatchQualityStatus::Frozen => "冻结",
        BatchQualityStatus::Scrapped => "报废",
    }
}

/// 把数据库质量状态转为领域枚举。
///
/// v9 数据库枚举里还有 “放行”，这里先按“合格”处理。
fn batch_status_from_db(value: &str) -> QualityResult<BatchQualityStatus> {
    match value {
        "待检" => Ok(BatchQualityStatus::PendingInspection),
        "合格" => Ok(BatchQualityStatus::Qualified),
        "冻结" => Ok(BatchQualityStatus::Frozen),
        "报废" => Ok(BatchQualityStatus::Scrapped),

        // 数据库有“放行”，业务上近似等于可用库存。
        "放行" => Ok(BatchQualityStatus::Qualified),

        _ => Err(QualityError::BatchQualityStatusInvalid),
    }
}

/// 批次质量动作转文字，用于 wms_batch_history.change_reason。
fn action_to_text(action: BatchQualityAction) -> &'static str {
    match action {
        BatchQualityAction::MarkPendingInspection => "标记待检",
        BatchQualityAction::Accept => "质量判定合格",
        BatchQualityAction::Freeze => "冻结批次",
        BatchQualityAction::Unfreeze => "解冻批次",
        BatchQualityAction::Scrap => "质量报废",
    }
}

/// change_reason 是 VARCHAR(100)，这里做防御性截断。
fn truncate_reason(value: &str) -> String {
    const MAX_LEN: usize = 100;

    if value.chars().count() <= MAX_LEN {
        return value.to_string();
    }

    value.chars().take(MAX_LEN).collect()
}

/// 把 reference_doc 映射到 wms_batch_history 的几个引用字段。
///
/// 当前约定：
/// - IL- 开头：inspection_lot_id
/// - QN- 开头：notification_id
/// - TRX- 或其他：transaction_id
fn split_reference_doc(
    reference_doc: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    let Some(value) = reference_doc else {
        return (None, None, None);
    };

    if value.starts_with("IL-") {
        (Some(value.to_string()), None, None)
    } else if value.starts_with("QN-") {
        (None, Some(value.to_string()), None)
    } else {
        (None, None, Some(value.to_string()))
    }
}

// =============================================================================
// BatchQualityRepository 实现
// =============================================================================

#[async_trait]
impl BatchQualityRepository for PostgresQualityStore {
    /// 锁定批次质量状态。
    ///
    /// 这里使用 SELECT ... FOR UPDATE。
    /// 质量判定、冻结、解冻、报废都必须先锁批次，避免并发状态错乱。
    async fn lock_batch_for_update(
        &self,
        batch_number: &BatchNumber,
    ) -> QualityResult<BatchQuality> {
        let row = sqlx::query(
            r#"
            SELECT
                batch_number,
                quality_status::text AS quality_status
            FROM wms.wms_batches
            WHERE batch_number = $1
            FOR UPDATE
            "#,
        )
            .bind(batch_number.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Err(QualityError::BusinessRuleViolation(format!(
                "批次不存在：{}",
                batch_number.as_str()
            )));
        };

        let status_text: String = row.get("quality_status");

        Ok(BatchQuality::new(
            BatchNumber::new(row.get::<String, _>("batch_number")),
            batch_status_from_db(&status_text)?,
        ))
    }

    /// 更新批次质量状态。
    async fn update_quality_status(
        &self,
        batch_number: &BatchNumber,
        status: BatchQualityStatus,
    ) -> QualityResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_batches
            SET
                quality_status = $2::mdm.quality_status,
                updated_at = NOW()
            WHERE batch_number = $1
            "#,
        )
            .bind(batch_number.as_str())
            .bind(batch_status_to_db(status))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(QualityError::BusinessRuleViolation(format!(
                "批次不存在：{}",
                batch_number.as_str()
            )));
        }

        Ok(())
    }

    /// 写入批次质量历史。
    ///
    /// 对应表：wms.wms_batch_history。
    async fn write_batch_history(
        &self,
        history: &BatchQualityHistory,
    ) -> QualityResult<()> {
        let old_status = history.old_status.map(batch_status_to_db);
        let new_status = batch_status_to_db(history.new_status);

        let reason = if history.reason.trim().is_empty() {
            action_to_text(history.action).to_string()
        } else {
            history.reason.clone()
        };

        let change_reason = truncate_reason(&reason);

        let (inspection_lot_id, notification_id, transaction_id) =
            split_reference_doc(history.reference_doc.as_deref());

        sqlx::query(
            r#"
            INSERT INTO wms.wms_batch_history (
                batch_number,
                old_quality_status,
                new_quality_status,
                change_reason,
                inspection_lot_id,
                notification_id,
                transaction_id,
                changed_by,
                changed_at
            )
            VALUES (
                $1,
                $2::mdm.quality_status,
                $3::mdm.quality_status,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9
            )
            "#,
        )
            .bind(history.batch_number.as_str())
            .bind(old_status)
            .bind(new_status)
            .bind(change_reason)
            .bind(inspection_lot_id)
            .bind(notification_id)
            .bind(transaction_id)
            .bind(history.operator.as_str())
            .bind(history.occurred_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
    }

    /// 查询批次质量状态。
    async fn get_batch_status(
        &self,
        batch_number: &BatchNumber,
    ) -> QualityResult<BatchQualityStatusView> {
        let row = sqlx::query(
            r#"
            SELECT
                batch_number,
                material_id,
                quality_status::text AS quality_status,
                current_stock
            FROM wms.wms_batches
            WHERE batch_number = $1
            "#,
        )
            .bind(batch_number.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Err(QualityError::BusinessRuleViolation(format!(
                "批次不存在：{}",
                batch_number.as_str()
            )));
        };

        let status_text: String = row.get("quality_status");
        let current_stock: i32 = row.get("current_stock");

        Ok(BatchQualityStatusView {
            batch_number: BatchNumber::new(row.get::<String, _>("batch_number")),
            material_id: MaterialId::new(row.get::<String, _>("material_id")),
            quality_status: batch_status_from_db(&status_text)?,
            current_qty: Decimal::from(current_stock),
        })
    }

    /// 查询批次质量历史。
    async fn list_batch_history(
        &self,
        batch_number: &BatchNumber,
        query: BatchHistoryQuery,
    ) -> QualityResult<Page<BatchQualityHistory>> {
        let page = query.page.page.max(1);
        let page_size = query.page.page_size.clamp(1, 200);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_batch_history
            WHERE batch_number = $1
            "#,
        )
            .bind(batch_number.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                batch_number,
                old_quality_status::text AS old_quality_status,
                new_quality_status::text AS new_quality_status,
                change_reason,
                inspection_lot_id,
                notification_id,
                transaction_id,
                changed_by,
                changed_at
            FROM wms.wms_batch_history
            WHERE batch_number = $1
            ORDER BY changed_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
            .bind(batch_number.as_str())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(rows.len());

        for row in rows {
            let old_status_text: Option<String> = row.get("old_quality_status");
            let new_status_text: String = row.get("new_quality_status");

            let new_status = batch_status_from_db(&new_status_text)?;
            let old_status = match old_status_text {
                Some(value) => Some(batch_status_from_db(&value)?),
                None => None,
            };

            let inspection_lot_id: Option<String> = row.get("inspection_lot_id");
            let notification_id: Option<String> = row.get("notification_id");
            let transaction_id: Option<String> = row.get("transaction_id");

            let reference_doc = inspection_lot_id
                .or(notification_id)
                .or(transaction_id);

            let reason: Option<String> = row.get("change_reason");
            let changed_by: Option<String> = row.get("changed_by");

            items.push(BatchQualityHistory {
                batch_number: BatchNumber::new(row.get::<String, _>("batch_number")),
                old_status,
                new_status,
                action: history_action_from_status(new_status),
                reason: reason.unwrap_or_else(|| "批次质量状态变更".to_string()),
                reference_doc,
                operator: Operator::new(changed_by.unwrap_or_else(|| "SYSTEM".to_string())),
                occurred_at: row.get::<OffsetDateTime, _>("changed_at"),
                remark: None,
            });
        }

        Ok(Page::new(items, total as u64, page, page_size))
    }
}

/// 从新状态反推一个历史动作。
///
/// 数据库历史表没有单独 action 字段，只有 change_reason 和状态。
/// 所以查询历史时先用 new_status 做一个近似映射。
fn history_action_from_status(status: BatchQualityStatus) -> BatchQualityAction {
    match status {
        BatchQualityStatus::PendingInspection => BatchQualityAction::MarkPendingInspection,
        BatchQualityStatus::Qualified => BatchQualityAction::Accept,
        BatchQualityStatus::Frozen => BatchQualityAction::Freeze,
        BatchQualityStatus::Scrapped => BatchQualityAction::Scrap,
    }
}

// =============================================================================
// QualityMasterRepository 实现
// =============================================================================

#[async_trait]
impl QualityMasterRepository for PostgresQualityStore {
    /// 检查物料是否存在且启用。
    ///
    /// v9 里 mdm_materials 使用 status 字段：
    /// - 正常
    /// - 停用
    /// - 冻结
    async fn material_exists_and_active(
        &self,
        material_id: &MaterialId,
    ) -> QualityResult<bool> {
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

    /// 查询检验特性。
    ///
    /// v9 的 mdm_inspection_chars 暂时没有 is_active 字段，
    /// 所以这里查询到就认为可用。
    async fn find_inspection_char(
        &self,
        char_id: &InspectionCharId,
    ) -> QualityResult<Option<InspectionCharSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT
                char_id,
                char_name,
                lower_limit,
                upper_limit,
                unit
            FROM mdm.mdm_inspection_chars
            WHERE char_id = $1
            "#,
        )
            .bind(char_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(InspectionCharSnapshot {
            char_id: InspectionCharId::new(row.get::<String, _>("char_id")),

            // v9 没有 char_code 字段，先用 char_id 作为 code。
            char_code: row.get::<String, _>("char_id"),

            char_name: row.get::<String, _>("char_name"),
            lower_limit: row.get::<Option<Decimal>, _>("lower_limit"),
            upper_limit: row.get::<Option<Decimal>, _>("upper_limit"),
            unit: row.get::<Option<String>, _>("unit"),

            // v9 没有 is_active 字段。
            is_active: true,
        }))
    }

    /// 查询不良代码。
    async fn find_defect_code(
        &self,
        defect_code: &DefectCode,
    ) -> QualityResult<Option<DefectCodeSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT
                defect_code,
                defect_name,
                description,
                is_active
            FROM mdm.mdm_defect_codes
            WHERE defect_code = $1
            "#,
        )
            .bind(defect_code.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let defect_name: String = row.get("defect_name");
        let description: Option<String> = row.get("description");

        Ok(Some(DefectCodeSnapshot {
            defect_code: DefectCode::new(row.get::<String, _>("defect_code")),
            description: description.unwrap_or(defect_name),
            is_active: row.get::<bool, _>("is_active"),
        }))
    }

    // =============================================================================
    // Inspection Lot 映射
    // =============================================================================

    /// 检验批类型映射到 v9 数据库 inspection_type。
    ///
    /// v9 只允许：
    /// - 来料检验
    /// - 过程检验
    /// - 最终检验
    fn inspection_lot_type_to_db(value: InspectionLotType) -> &'static str {
        match value {
            InspectionLotType::PurchaseReceipt => "来料检验",
            InspectionLotType::ProductionReceipt => "最终检验",
            InspectionLotType::StockRecheck => "过程检验",
            InspectionLotType::CustomerReturn => "最终检验",
            InspectionLotType::Manual => "过程检验",
        }
    }

    /// v9 数据库 inspection_type 映射回领域类型。
    fn inspection_lot_type_from_db(value: &str) -> InspectionLotType {
        match value {
            "来料检验" => InspectionLotType::PurchaseReceipt,
            "最终检验" => InspectionLotType::ProductionReceipt,
            "过程检验" => InspectionLotType::Manual,
            _ => InspectionLotType::Manual,
        }
    }

    /// 检验批领域状态映射到数据库 lot_status。
    ///
    /// 注意：
    /// v9 的 lot_status 使用 mdm.quality_status：
    /// 待检 / 合格 / 冻结 / 报废 / 放行。
    ///
    /// 领域里的 CREATED / IN_PROGRESS / RESULT_ENTERED 等细状态，
    /// 会额外放到 inspection_result JSONB 的 app_status 字段。
    fn lot_status_to_db(status: InspectionLotStatus, decision: Option<InspectionDecision>) -> &'static str {
        match decision {
            Some(InspectionDecision::Accept) => "合格",
            Some(InspectionDecision::Freeze) => "冻结",
            Some(InspectionDecision::Scrap) => "报废",
            None => match status {
                InspectionLotStatus::Created => "待检",
                InspectionLotStatus::InProgress => "待检",
                InspectionLotStatus::ResultEntered => "待检",
                InspectionLotStatus::Decided => "待检",
                InspectionLotStatus::Closed => "合格",
                InspectionLotStatus::Cancelled => "冻结",
            },
        }
    }

    /// 从 JSONB 里的 app_status 恢复领域状态。
    ///
    /// 如果历史数据没有 app_status，则根据数据库 lot_status 做一个保守推断。
    fn inspection_lot_status_from_json_or_db(
        meta: &Value,
        lot_status_text: &str,
    ) -> InspectionLotStatus {
        match meta.get("app_status").and_then(|v| v.as_str()) {
            Some("CREATED") => InspectionLotStatus::Created,
            Some("IN_PROGRESS") => InspectionLotStatus::InProgress,
            Some("RESULT_ENTERED") => InspectionLotStatus::ResultEntered,
            Some("DECIDED") => InspectionLotStatus::Decided,
            Some("CLOSED") => InspectionLotStatus::Closed,
            Some("CANCELLED") => InspectionLotStatus::Cancelled,
            _ => match lot_status_text {
                "待检" => InspectionLotStatus::Created,
                "合格" => InspectionLotStatus::Decided,
                "冻结" => InspectionLotStatus::Decided,
                "报废" => InspectionLotStatus::Decided,
                "放行" => InspectionLotStatus::Decided,
                _ => InspectionLotStatus::Created,
            },
        }
    }

    /// 领域状态转字符串，写入 inspection_result JSONB。
    fn inspection_lot_status_to_code(status: InspectionLotStatus) -> &'static str {
        match status {
            InspectionLotStatus::Created => "CREATED",
            InspectionLotStatus::InProgress => "IN_PROGRESS",
            InspectionLotStatus::ResultEntered => "RESULT_ENTERED",
            InspectionLotStatus::Decided => "DECIDED",
            InspectionLotStatus::Closed => "CLOSED",
            InspectionLotStatus::Cancelled => "CANCELLED",
        }
    }

    /// 质量判定转字符串。
    fn inspection_decision_to_code(decision: InspectionDecision) -> &'static str {
        match decision {
            InspectionDecision::Accept => "ACCEPT",
            InspectionDecision::Freeze => "FREEZE",
            InspectionDecision::Scrap => "SCRAP",
        }
    }

    /// 字符串转质量判定。
    fn inspection_decision_from_code(value: &str) -> Option<InspectionDecision> {
        match value {
            "ACCEPT" => Some(InspectionDecision::Accept),
            "FREEZE" => Some(InspectionDecision::Freeze),
            "SCRAP" => Some(InspectionDecision::Scrap),
            _ => None,
        }
    }

    /// 把 InspectionLot 中 v9 表没有的字段打包进 JSONB。
    fn inspection_lot_to_meta(lot: &InspectionLot) -> Value {
        json!({
        "app_status": inspection_lot_status_to_code(lot.status),
        "decision": lot.decision.map(inspection_decision_to_code),
        "source_transaction_id": lot.source_transaction_id,
        "source_doc": lot.source_doc,
        "quantity": lot.quantity,
        "sample_qty": lot.sample_qty,
        "created_by": lot.created_by.as_str(),
        "decided_by": lot.decided_by.as_ref().map(|x| x.as_str().to_string()),
        "created_at": lot.created_at,
        "inspected_at": lot.inspected_at,
        "decided_at": lot.decided_at,
        "closed_at": lot.closed_at,
        "remark": lot.remark
    })
    }

    /// 从数据库行恢复 InspectionLot。
    fn inspection_lot_from_row(row: &sqlx::postgres::PgRow) -> QualityResult<InspectionLot> {
        let meta: Value = row
            .try_get::<Value, _>("inspection_result")
            .unwrap_or_else(|_| json!({}));

        let lot_status_text: String = row.get("lot_status");
        let status = inspection_lot_status_from_json_or_db(&meta, &lot_status_text);

        let decision = meta
            .get("decision")
            .and_then(|v| v.as_str())
            .and_then(inspection_decision_from_code)
            .or_else(|| match lot_status_text.as_str() {
                "合格" | "放行" => Some(InspectionDecision::Accept),
                "冻结" => Some(InspectionDecision::Freeze),
                "报废" => Some(InspectionDecision::Scrap),
                _ => None,
            });

        let quantity = meta
            .get("quantity")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(Decimal::ZERO);

        let sample_qty = meta
            .get("sample_qty")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(Decimal::ZERO);

        let source_transaction_id = meta
            .get("source_transaction_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let source_doc = meta
            .get("source_doc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let created_by = meta
            .get("created_by")
            .and_then(|v| v.as_str())
            .unwrap_or("SYSTEM");

        let decided_by = meta
            .get("decided_by")
            .and_then(|v| v.as_str())
            .map(Operator::new);

        let remark = meta
            .get("remark")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let created_at: OffsetDateTime = row.get("created_at");
        let inspection_date: Option<OffsetDateTime> = row.get("inspection_date");

        Ok(InspectionLot {
            id: InspectionLotId::new(row.get::<String, _>("inspection_lot_id")),
            lot_type: inspection_lot_type_from_db(&row.get::<String, _>("inspection_type")),
            material_id: MaterialId::new(row.get::<String, _>("material_id")),
            batch_number: BatchNumber::new(row.get::<String, _>("batch_number")),
            source_transaction_id,
            source_doc,
            quantity,
            sample_qty,
            status,
            decision,
            created_by: Operator::new(created_by),
            inspected_by: row
                .get::<Option<String>, _>("inspector")
                .map(Operator::new),
            decided_by,
            created_at,
            inspected_at: inspection_date,
            decided_at: meta
                .get("decided_at")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            closed_at: meta
                .get("closed_at")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            remark,
        })
    }
}
// =============================================================================
// Inspection Result 映射
// =============================================================================

/// 检验结果状态映射到 v9 数据库 result。
fn inspection_result_status_to_db(status: InspectionResultStatus) -> &'static str {
    match status {
        InspectionResultStatus::Pass => "合格",
        InspectionResultStatus::Fail => "不合格",
        InspectionResultStatus::NotApplicable => "让步接收",
    }
}

/// v9 数据库 result 映射回领域状态。
fn inspection_result_status_from_db(value: &str) -> InspectionResultStatus {
    match value {
        "合格" => InspectionResultStatus::Pass,
        "不合格" => InspectionResultStatus::Fail,
        "让步接收" => InspectionResultStatus::NotApplicable,
        _ => InspectionResultStatus::NotApplicable,
    }
}

/// 把 v9 表没有的检验结果字段打包到 remarks 文本中。
///
/// remarks 仍然是 TEXT，但内容用 JSON 存，方便后续扩展。
fn inspection_result_to_remarks_json(result: &InspectionResult) -> String {
    json!({
        "qualitative_result": result.qualitative_result,
        "lower_limit": result.lower_limit,
        "upper_limit": result.upper_limit,
        "unit": result.unit,
        "defect_code": result.defect_code.as_ref().map(|x| x.as_str().to_string()),
        "defect_qty": result.defect_qty,
        "remark": result.remark
    })
        .to_string()
}

/// 从 remarks TEXT 尝试解析 JSON。
///
/// 兼容历史普通文本备注。
fn parse_result_remarks(value: Option<String>) -> Value {
    let Some(text) = value else {
        return json!({});
    };

    serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
        json!({
            "remark": text
        })
    })
}

/// 从数据库行恢复 InspectionResult。
fn inspection_result_from_row(row: &sqlx::postgres::PgRow) -> QualityResult<InspectionResult> {
    let remarks_meta = parse_result_remarks(row.get::<Option<String>, _>("remarks"));

    let result_text: String = row.get("result");
    let result_status = inspection_result_status_from_db(&result_text);

    let defect_code = remarks_meta
        .get("defect_code")
        .and_then(|v| v.as_str())
        .map(DefectCode::new);

    let defect_qty = remarks_meta
        .get("defect_qty")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(Decimal::ZERO);

    let qualitative_result = remarks_meta
        .get("qualitative_result")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let lower_limit = remarks_meta
        .get("lower_limit")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let upper_limit = remarks_meta
        .get("upper_limit")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let unit = remarks_meta
        .get("unit")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let remark = remarks_meta
        .get("remark")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(InspectionResult {
        id: InspectionResultId::new(row.get::<i64, _>("id").to_string()),
        inspection_lot_id: InspectionLotId::new(row.get::<String, _>("inspection_lot_id")),
        char_id: InspectionCharId::new(row.get::<String, _>("char_id")),
        measured_value: row.get::<Option<Decimal>, _>("measured_value"),
        qualitative_result,
        lower_limit,
        upper_limit,
        unit,
        result_status,
        defect_code,
        defect_qty,
        inspector: Operator::new(
            row.get::<Option<String>, _>("inspected_by")
                .unwrap_or_else(|| "SYSTEM".to_string()),
        ),
        inspected_at: row.get::<OffsetDateTime, _>("inspected_at"),
        remark,
    })
}
// =============================================================================
// Quality Notification 映射
// =============================================================================

/// 领域严重等级映射到 v9 数据库 severity。
fn notification_severity_to_db(value: QualityNotificationSeverity) -> &'static str {
    match value {
        QualityNotificationSeverity::Low => "一般",
        QualityNotificationSeverity::Medium => "一般",
        QualityNotificationSeverity::High => "严重",
        QualityNotificationSeverity::Critical => "紧急",
    }
}

/// v9 数据库 severity 映射回领域严重等级。
fn notification_severity_from_db(value: Option<String>) -> QualityNotificationSeverity {
    match value.as_deref() {
        Some("一般") => QualityNotificationSeverity::Medium,
        Some("严重") => QualityNotificationSeverity::High,
        Some("紧急") => QualityNotificationSeverity::Critical,
        _ => QualityNotificationSeverity::Medium,
    }
}

/// 领域通知状态映射到 v9 数据库 status。
///
/// v9 只支持：处理中 / 已关闭 / 已报废。
fn notification_status_to_db(value: QualityNotificationStatus) -> &'static str {
    match value {
        QualityNotificationStatus::Open => "处理中",
        QualityNotificationStatus::InProgress => "处理中",
        QualityNotificationStatus::Resolved => "处理中",
        QualityNotificationStatus::Closed => "已关闭",
        QualityNotificationStatus::Cancelled => "已关闭",
    }
}

/// v9 数据库 status 映射回领域状态。
fn notification_status_from_db(value: &str) -> QualityNotificationStatus {
    match value {
        "处理中" => QualityNotificationStatus::InProgress,
        "已关闭" => QualityNotificationStatus::Closed,
        "已报废" => QualityNotificationStatus::Closed,
        _ => QualityNotificationStatus::Open,
    }
}

/// 从数据库行恢复 QualityNotification。
fn quality_notification_from_row(row: &sqlx::postgres::PgRow) -> QualityResult<QualityNotification> {
    let status_text: String = row.get("status");

    Ok(QualityNotification {
        id: QualityNotificationId::new(row.get::<String, _>("notification_id")),
        source_type: "INSPECTION_LOT".to_string(),
        source_id: row
            .get::<Option<String>, _>("inspection_lot_id")
            .unwrap_or_default(),
        material_id: MaterialId::new(row.get::<String, _>("material_id")),
        batch_number: BatchNumber::new(
            row.get::<Option<String>, _>("batch_number")
                .unwrap_or_default(),
        ),
        defect_code: row
            .get::<Option<String>, _>("defect_code")
            .map(DefectCode::new),
        defect_qty: Decimal::ZERO,
        severity: notification_severity_from_db(row.get::<Option<String>, _>("severity")),
        status: notification_status_from_db(&status_text),
        description: row.get::<String, _>("problem_description"),
        owner: row
            .get::<Option<String>, _>("responsible_person")
            .map(Operator::new),
        root_cause: row.get::<Option<String>, _>("root_cause"),
        corrective_action: row.get::<Option<String>, _>("corrective_action"),
        created_by: Operator::new("SYSTEM"),
        created_at: row.get::<OffsetDateTime, _>("created_at"),
        closed_by: None,
        closed_at: row.get::<Option<OffsetDateTime>, _>("closed_at"),
        remark: None,
    })
}
// =============================================================================
// InspectionLotRepository 实现
// =============================================================================

#[async_trait]
impl InspectionLotRepository for PostgresQualityStore {
    /// 创建检验批。
    async fn create(&self, lot: &InspectionLot) -> QualityResult<InspectionLotId> {
        let meta = inspection_lot_to_meta(lot);

        sqlx::query(
            r#"
            INSERT INTO wms.wms_inspection_lots (
                inspection_lot_id,
                material_id,
                batch_number,
                inspection_type,
                lot_status,
                inspection_date,
                inspector,
                inspection_result,
                created_at,
                updated_at
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5::mdm.quality_status,
                $6,
                $7,
                $8,
                $9,
                NOW()
            )
            "#,
        )
            .bind(lot.id.as_str())
            .bind(lot.material_id.as_str())
            .bind(lot.batch_number.as_str())
            .bind(inspection_lot_type_to_db(lot.lot_type))
            .bind(lot_status_to_db(lot.status, lot.decision))
            .bind(lot.inspected_at)
            .bind(lot.inspected_by.as_ref().map(|x| x.as_str().to_string()))
            .bind(meta)
            .bind(lot.created_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(lot.id.clone())
    }

    /// 按 ID 查询检验批。
    async fn find_by_id(&self, lot_id: &InspectionLotId) -> QualityResult<Option<InspectionLot>> {
        let row = sqlx::query(
            r#"
            SELECT
                inspection_lot_id,
                material_id,
                batch_number,
                inspection_type,
                lot_status::text AS lot_status,
                inspection_date,
                inspector,
                inspection_result,
                created_at,
                updated_at
            FROM wms.wms_inspection_lots
            WHERE inspection_lot_id = $1
            "#,
        )
            .bind(lot_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            Some(row) => Ok(Some(inspection_lot_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 分页查询检验批。
    async fn list(&self, query: InspectionLotQuery) -> QualityResult<Page<InspectionLotSummary>> {
        let page = query.page.page.max(1);
        let page_size = query.page.page_size.clamp(1, 200);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let lot_type = query.lot_type.map(inspection_lot_type_to_db);
        let lot_status = query.status.map(|status| lot_status_to_db(status, None));

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_inspection_lots
            WHERE ($1::text IS NULL OR inspection_type = $1)
              AND ($2::text IS NULL OR lot_status::text = $2)
              AND ($3::text IS NULL OR material_id = $3)
              AND ($4::text IS NULL OR batch_number = $4)
              AND ($5::timestamptz IS NULL OR created_at >= $5)
              AND ($6::timestamptz IS NULL OR created_at < $6)
            "#,
        )
            .bind(lot_type)
            .bind(lot_status)
            .bind(query.material_id.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.batch_number.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.date_from)
            .bind(query.date_to)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                inspection_lot_id,
                material_id,
                batch_number,
                inspection_type,
                lot_status::text AS lot_status,
                inspection_date,
                inspector,
                inspection_result,
                created_at,
                updated_at
            FROM wms.wms_inspection_lots
            WHERE ($1::text IS NULL OR inspection_type = $1)
              AND ($2::text IS NULL OR lot_status::text = $2)
              AND ($3::text IS NULL OR material_id = $3)
              AND ($4::text IS NULL OR batch_number = $4)
              AND ($5::timestamptz IS NULL OR created_at >= $5)
              AND ($6::timestamptz IS NULL OR created_at < $6)
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
            .bind(lot_type)
            .bind(lot_status)
            .bind(query.material_id.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.batch_number.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.date_from)
            .bind(query.date_to)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(rows.len());

        for row in rows {
            let lot = inspection_lot_from_row(&row)?;

            items.push(InspectionLotSummary {
                id: lot.id,
                lot_type: lot.lot_type,
                material_id: lot.material_id,
                batch_number: lot.batch_number,
                quantity: lot.quantity,
                sample_qty: lot.sample_qty,
                status: lot.status,
                decision: lot.decision,
                created_at: lot.created_at,
            });
        }

        Ok(Page::new(items, total as u64, page, page_size))
    }

    /// 锁定检验批。
    ///
    /// 质量判定、录入结果时使用。
    async fn lock_by_id(&self, lot_id: &InspectionLotId) -> QualityResult<InspectionLot> {
        let row = sqlx::query(
            r#"
            SELECT
                inspection_lot_id,
                material_id,
                batch_number,
                inspection_type,
                lot_status::text AS lot_status,
                inspection_date,
                inspector,
                inspection_result,
                created_at,
                updated_at
            FROM wms.wms_inspection_lots
            WHERE inspection_lot_id = $1
            FOR UPDATE
            "#,
        )
            .bind(lot_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Err(QualityError::InspectionLotNotFound);
        };

        inspection_lot_from_row(&row)
    }

    /// 更新检验批。
    async fn update(&self, lot: &InspectionLot) -> QualityResult<()> {
        let meta = inspection_lot_to_meta(lot);

        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inspection_lots
            SET
                lot_status = $2::mdm.quality_status,
                inspection_date = $3,
                inspector = $4,
                inspection_result = $5,
                updated_at = NOW()
            WHERE inspection_lot_id = $1
            "#,
        )
            .bind(lot.id.as_str())
            .bind(lot_status_to_db(lot.status, lot.decision))
            .bind(lot.inspected_at)
            .bind(lot.inspected_by.as_ref().map(|x| x.as_str().to_string()))
            .bind(meta)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(QualityError::InspectionLotNotFound);
        }

        Ok(())
    }

    /// 检查同一批次是否已有未关闭检验批。
    async fn exists_open_by_batch(&self, batch_number: &BatchNumber) -> QualityResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM wms.wms_inspection_lots
                WHERE batch_number = $1
                  AND lot_status IN ('待检', '冻结')
            )
            "#,
        )
            .bind(batch_number.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(exists)
    }
}
// =============================================================================
// InspectionResultRepository 实现
// =============================================================================

#[async_trait]
impl InspectionResultRepository for PostgresQualityStore {
    /// 创建单条检验结果。
    async fn create(&self, result: &InspectionResult) -> QualityResult<InspectionResultId> {
        let remarks = inspection_result_to_remarks_json(result);

        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO wms.wms_inspection_results (
                inspection_lot_id,
                char_id,
                measured_value,
                result,
                remarks,
                inspected_by,
                inspected_at
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            )
            RETURNING id
            "#,
        )
            .bind(result.inspection_lot_id.as_str())
            .bind(result.char_id.as_str())
            .bind(result.measured_value)
            .bind(inspection_result_status_to_db(result.result_status))
            .bind(remarks)
            .bind(result.inspector.as_str())
            .bind(result.inspected_at)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(InspectionResultId::new(id.to_string()))
    }

    /// 批量创建检验结果。
    async fn batch_create(
        &self,
        results: &[InspectionResult],
    ) -> QualityResult<Vec<InspectionResultId>> {
        let mut ids = Vec::with_capacity(results.len());

        // MVP 先逐条插入，便于清晰处理错误。
        // 后续如果性能压力大，再换成 QueryBuilder 批量 INSERT。
        for result in results {
            let id = self.create(result).await?;
            ids.push(id);
        }

        Ok(ids)
    }

    /// 查询某个检验批下所有检验结果。
    async fn find_by_lot_id(
        &self,
        lot_id: &InspectionLotId,
    ) -> QualityResult<Vec<InspectionResult>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                inspection_lot_id,
                char_id,
                measured_value,
                result,
                remarks,
                inspected_by,
                inspected_at
            FROM wms.wms_inspection_results
            WHERE inspection_lot_id = $1
            ORDER BY id ASC
            "#,
        )
            .bind(lot_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(rows.len());

        for row in rows {
            items.push(inspection_result_from_row(&row)?);
        }

        Ok(items)
    }

    /// 当前检验批是否已有检验结果。
    async fn has_any_result(&self, lot_id: &InspectionLotId) -> QualityResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM wms.wms_inspection_results
                WHERE inspection_lot_id = $1
            )
            "#,
        )
            .bind(lot_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(exists)
    }

    /// 当前检验批是否存在失败项。
    async fn has_failed_result(&self, lot_id: &InspectionLotId) -> QualityResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM wms.wms_inspection_results
                WHERE inspection_lot_id = $1
                  AND result = '不合格'
            )
            "#,
        )
            .bind(lot_id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(exists)
    }
}
// =============================================================================
// QualityNotificationRepository 实现
// =============================================================================

#[async_trait]
impl QualityNotificationRepository for PostgresQualityStore {
    /// 创建质量通知。
    async fn create(
        &self,
        notification: &QualityNotification,
    ) -> QualityResult<QualityNotificationId> {
        sqlx::query(
            r#"
            INSERT INTO wms.wms_quality_notifications (
                notification_id,
                inspection_lot_id,
                material_id,
                batch_number,
                defect_code,
                problem_description,
                severity,
                root_cause,
                corrective_action,
                responsible_person,
                status,
                created_at,
                closed_at
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
                $9,
                $10,
                $11,
                $12,
                $13
            )
            "#,
        )
            .bind(notification.id.as_str())
            .bind(if notification.source_type == "INSPECTION_LOT" {
                Some(notification.source_id.clone())
            } else {
                None
            })
            .bind(notification.material_id.as_str())
            .bind(notification.batch_number.as_str())
            .bind(notification.defect_code.as_ref().map(|x| x.as_str().to_string()))
            .bind(&notification.description)
            .bind(notification_severity_to_db(notification.severity))
            .bind(&notification.root_cause)
            .bind(&notification.corrective_action)
            .bind(notification.owner.as_ref().map(|x| x.as_str().to_string()))
            .bind(notification_status_to_db(notification.status))
            .bind(notification.created_at)
            .bind(notification.closed_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(notification.id.clone())
    }

    /// 按 ID 查询质量通知。
    async fn find_by_id(
        &self,
        notification_id: &QualityNotificationId,
    ) -> QualityResult<Option<QualityNotification>> {
        let row = sqlx::query(
            r#"
            SELECT
                notification_id,
                inspection_lot_id,
                material_id,
                batch_number,
                defect_code,
                problem_description,
                severity,
                root_cause,
                corrective_action,
                responsible_person,
                status,
                created_at,
                closed_at
            FROM wms.wms_quality_notifications
            WHERE notification_id = $1
            "#,
        )
            .bind(notification_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        match row {
            Some(row) => Ok(Some(quality_notification_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 分页查询质量通知。
    async fn list(
        &self,
        query: QualityNotificationQuery,
    ) -> QualityResult<Page<QualityNotificationSummary>> {
        let page = query.page.page.max(1);
        let page_size = query.page.page_size.clamp(1, 200);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let status = query.status.map(notification_status_to_db);
        let severity = query.severity.map(notification_severity_to_db);

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM wms.wms_quality_notifications
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR severity = $2)
              AND ($3::text IS NULL OR material_id = $3)
              AND ($4::text IS NULL OR batch_number = $4)
              AND ($5::text IS NULL OR responsible_person = $5)
              AND ($6::timestamptz IS NULL OR created_at >= $6)
              AND ($7::timestamptz IS NULL OR created_at < $7)
            "#,
        )
            .bind(status)
            .bind(severity)
            .bind(query.material_id.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.batch_number.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.owner)
            .bind(query.date_from)
            .bind(query.date_to)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let rows = sqlx::query(
            r#"
            SELECT
                notification_id,
                inspection_lot_id,
                material_id,
                batch_number,
                defect_code,
                problem_description,
                severity,
                root_cause,
                corrective_action,
                responsible_person,
                status,
                created_at,
                closed_at
            FROM wms.wms_quality_notifications
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR severity = $2)
              AND ($3::text IS NULL OR material_id = $3)
              AND ($4::text IS NULL OR batch_number = $4)
              AND ($5::text IS NULL OR responsible_person = $5)
              AND ($6::timestamptz IS NULL OR created_at >= $6)
              AND ($7::timestamptz IS NULL OR created_at < $7)
            ORDER BY created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
            .bind(status)
            .bind(severity)
            .bind(query.material_id.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.batch_number.as_ref().map(|x| x.as_str().to_string()))
            .bind(query.owner)
            .bind(query.date_from)
            .bind(query.date_to)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut items = Vec::with_capacity(rows.len());

        for row in rows {
            let notification = quality_notification_from_row(&row)?;

            items.push(QualityNotificationSummary {
                id: notification.id,
                material_id: notification.material_id,
                batch_number: notification.batch_number,
                severity: notification.severity,
                status: notification.status,
                description: notification.description,
                created_at: notification.created_at,
            });
        }

        Ok(Page::new(items, total as u64, page, page_size))
    }

    /// 更新质量通知。
    async fn update(&self, notification: &QualityNotification) -> QualityResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_quality_notifications
            SET
                problem_description = $2,
                severity = $3,
                root_cause = $4,
                corrective_action = $5,
                responsible_person = $6,
                status = $7,
                closed_at = $8
            WHERE notification_id = $1
            "#,
        )
            .bind(notification.id.as_str())
            .bind(&notification.description)
            .bind(notification_severity_to_db(notification.severity))
            .bind(&notification.root_cause)
            .bind(&notification.corrective_action)
            .bind(notification.owner.as_ref().map(|x| x.as_str().to_string()))
            .bind(notification_status_to_db(notification.status))
            .bind(notification.closed_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(QualityError::QualityNotificationNotFound);
        }

        Ok(())
    }
}