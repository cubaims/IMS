use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};

use crate::application::common::Page;
use crate::application::errors::InventoryCountApplicationError;
use crate::application::inventory_count_model::{
    InventoryCountScopeFilter,
    ListInventoryCountsInput,
};
use crate::application::inventory_count_repository::{
    InventoryCountRepository,
    InventoryCountSummary,
};
use crate::domain::{
    InventoryCount,
    InventoryCountLine,
    InventoryCountLineStatus,
    InventoryCountMovementType,
    InventoryCountPostedTransaction,
    InventoryCountScope,
    InventoryCountStatus,
    InventoryCountType,
};

/// PostgreSQL 盘点仓储实现
///
/// 不做读写分离：
/// 查询、创建、更新、过账都在这个 Repository 里实现。
#[derive(Clone)]
pub struct PostgresInventoryCountRepository {
    pool: PgPool,
}

impl PostgresInventoryCountRepository {
    /// 调用数据库库存过账函数
    ///
    /// 这里集中封装 wms.post_inventory_transaction()。
    /// 如果数据库函数签名变化，只需要改这里，不影响 service / domain。
    async fn call_post_inventory_transaction(
        &self,
        movement_type: &str,
        material_id: &str,
        batch_number: Option<&str>,
        quantity: Decimal,
        from_bin: Option<&str>,
        to_bin: Option<&str>,
        quality_status: Option<&str>,
        operator: &str,
        posting_date: DateTime<Utc>,
        reference_doc: Option<&str>,
        reference_line_no: Option<i32>,
        remark: Option<&str>,
    ) -> Result<String, InventoryCountApplicationError> {
        let row = sqlx::query(
            r#"
            SELECT wms.post_inventory_transaction(
                p_movement_type     => $1,
                p_material_id       => $2,
                p_batch_number      => $3,
                p_quantity          => $4,
                p_from_bin          => $5,
                p_to_bin            => $6,
                p_quality_status    => $7,
                p_operator          => $8,
                p_posting_date      => $9,
                p_reference_doc     => $10,
                p_reference_line_no => $11,
                p_remark            => $12
            ) AS transaction_id
            "#,
        )
            .bind(movement_type)
            .bind(material_id)
            .bind(batch_number)
            .bind(quantity)
            .bind(from_bin)
            .bind(to_bin)
            .bind(quality_status)
            .bind(operator)
            .bind(posting_date)
            .bind(reference_doc)
            .bind(reference_line_no)
            .bind(remark)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| {
                InventoryCountApplicationError::DifferencePostFailed(format!(
                    "调用 wms.post_inventory_transaction({movement_type}) 失败: {err}"
                ))
            })?;

        let transaction_id: String = row
            .try_get("transaction_id")
            .map_err(InventoryCountApplicationError::database)?;

        Ok(transaction_id)
    }
}

/* -------------------------------------------------------------------------- */
/*                               数据库 Row 结构                               */
/* -------------------------------------------------------------------------- */

/// 盘点单头 Row
///
/// 字段名按模板里的 wms.wms_inventory_count_h 设计。
/// 如果你的实际 SQL schema 字段名略有不同，只需要调整这里和 SQL 里的列名。
#[derive(Debug, sqlx::FromRow)]
struct InventoryCountHeaderRow {
    pub count_doc_id: String,
    pub count_type: String,
    pub count_scope: String,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub status: String,

    pub created_by: String,
    pub approved_by: Option<String>,
    pub posted_by: Option<String>,

    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub posted_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,

    pub remark: Option<String>,
}

/// 盘点明细 Row
#[derive(Debug, sqlx::FromRow)]
struct InventoryCountLineRow {
    pub count_doc_id: String,
    pub line_no: i32,

    pub material_id: String,
    pub bin_code: String,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,

    pub system_qty: Decimal,
    pub counted_qty: Option<Decimal>,
    pub difference_qty: Option<Decimal>,
    pub difference_reason: Option<String>,

    pub movement_type: Option<String>,
    pub transaction_id: Option<String>,

    pub status: String,
    pub remark: Option<String>,
}

/// 盘点列表摘要 Row
#[derive(Debug, sqlx::FromRow)]
struct InventoryCountSummaryRow {
    pub count_doc_id: String,
    pub count_type: String,
    pub count_scope: String,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub status: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,

    pub line_count: i64,
    pub difference_line_count: i64,
    pub remark: Option<String>,
}

/* -------------------------------------------------------------------------- */
/*                              Row -> Domain 映射                             */
/* -------------------------------------------------------------------------- */

impl TryFrom<InventoryCountHeaderRow> for InventoryCount {
    type Error = InventoryCountApplicationError;

    fn try_from(row: InventoryCountHeaderRow) -> Result<Self, Self::Error> {
        Ok(Self {
            count_doc_id: row.count_doc_id,
            count_type: parse_count_type(&row.count_type)?,
            count_scope: parse_count_scope(&row.count_scope)?,
            zone_code: row.zone_code,
            bin_code: row.bin_code,
            material_id: row.material_id,
            batch_number: row.batch_number,
            status: parse_count_status(&row.status)?,
            created_by: row.created_by,
            approved_by: row.approved_by,
            posted_by: row.posted_by,
            created_at: row.created_at,
            approved_at: row.approved_at,
            posted_at: row.posted_at,
            closed_at: row.closed_at,
            remark: row.remark,
            lines: Vec::new(),
        })
    }
}

impl TryFrom<InventoryCountLineRow> for InventoryCountLine {
    type Error = InventoryCountApplicationError;

    fn try_from(row: InventoryCountLineRow) -> Result<Self, Self::Error> {
        Ok(Self {
            count_doc_id: row.count_doc_id,
            line_no: row.line_no,
            material_id: row.material_id,
            bin_code: row.bin_code,
            batch_number: row.batch_number,
            quality_status: row.quality_status,
            system_qty: row.system_qty,
            counted_qty: row.counted_qty,
            difference_qty: row.difference_qty,
            difference_reason: row.difference_reason,
            movement_type: parse_movement_type_opt(row.movement_type.as_deref())?,
            transaction_id: row.transaction_id,
            status: parse_line_status(&row.status)?,
            remark: row.remark,
        })
    }
}

impl TryFrom<InventoryCountSummaryRow> for InventoryCountSummary {
    type Error = InventoryCountApplicationError;

    fn try_from(row: InventoryCountSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            count_doc_id: row.count_doc_id,
            count_type: parse_count_type(&row.count_type)?,
            count_scope: parse_count_scope(&row.count_scope)?,
            zone_code: row.zone_code,
            bin_code: row.bin_code,
            material_id: row.material_id,
            batch_number: row.batch_number,
            status: parse_count_status(&row.status)?,
            created_by: row.created_by,
            created_at: row.created_at,
            line_count: row.line_count,
            difference_line_count: row.difference_line_count,
            remark: row.remark,
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                              Repository 实现                                */
/* -------------------------------------------------------------------------- */

#[async_trait::async_trait]
impl InventoryCountRepository for PostgresInventoryCountRepository {
    /// 生成盘点单号
    ///
    /// 这里用数据库时间 + sequence。
    /// 你也可以替换为项目已有的单号服务。
    /// 取消盘点单
    /// 盘点过账事务方法
    ///
    /// 这是盘点过账的最终实现。
    /// 所有 SQL 都在同一个事务里执行。
    async fn post_count_transactional(
        &self,
        count_doc_id: &str,
        operator: &str,
        posting_date: DateTime<Utc>,
        remark: Option<&str>,
    ) -> Result<InventoryCountPostingResult, InventoryCountApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(InventoryCountApplicationError::database)?;

        // 1. 锁定盘点单头
        let header_row = sqlx::query_as::<_, InventoryCountHeaderRow>(
            r#"
            SELECT
                count_doc_id,
                count_type,
                count_scope,
                zone_code,
                bin_code,
                material_id,
                batch_number,
                status,
                created_by,
                approved_by,
                posted_by,
                created_at,
                approved_at,
                posted_at,
                closed_at,
                remark
            FROM wms.wms_inventory_count_h
            WHERE count_doc_id = $1
            FOR UPDATE
            "#,
        )
            .bind(count_doc_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(InventoryCountApplicationError::database)?
            .ok_or(InventoryCountApplicationError::CountNotFound)?;

        let mut count = InventoryCount::try_from(header_row)?;

        // 2. 校验状态必须是 APPROVED
        if !count.status.can_post() {
            return Err(InventoryCountApplicationError::StatusInvalid);
        }

        // 3. 锁定盘点明细
        let line_rows = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            SELECT
                count_doc_id,
                line_no,
                material_id,
                bin_code,
                batch_number,
                quality_status,
                system_qty,
                counted_qty,
                difference_qty,
                difference_reason,
                movement_type,
                transaction_id,
                status,
                remark
            FROM wms.wms_inventory_count_d
            WHERE count_doc_id = $1
            ORDER BY line_no
            FOR UPDATE
            "#,
        )
            .bind(count_doc_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if line_rows.is_empty() {
            return Err(InventoryCountApplicationError::NoLines);
        }

        let mut lines = line_rows
            .into_iter()
            .map(InventoryCountLine::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let mut transactions = Vec::new();

        // 4. 逐行处理差异
        for line in lines.iter_mut() {
            let difference_qty = line
                .difference_qty
                .ok_or(InventoryCountApplicationError::LineNotCounted)?;

            // 无差异：不生成库存事务，也不回写 transaction_id
            if difference_qty == Decimal::ZERO {
                continue;
            }

            // 有差异的行必须有差异原因
            line.validate_before_submit()?;

            let posting_qty = line.posting_qty();

            if posting_qty <= Decimal::ZERO {
                return Err(InventoryCountApplicationError::DifferencePostFailed(
                    format!("过账数量必须大于 0，line_no={}", line.line_no),
                ));
            }

            let (movement_type, from_bin, to_bin) = if line.is_gain() {
                // 盘盈：701，增加到当前货位
                ("701", None, Some(line.bin_code.as_str()))
            } else if line.is_loss() {
                // 盘亏：702，从当前货位扣减
                ("702", Some(line.bin_code.as_str()), None)
            } else {
                // 理论上不会走到这里，前面已经处理 difference_qty == 0
                continue;
            };

            let transaction_id = self
                .call_post_inventory_transaction_tx(
                    &mut tx,
                    movement_type,
                    &line.material_id,
                    line.batch_number.as_deref(),
                    posting_qty,
                    from_bin,
                    to_bin,
                    line.quality_status.as_deref(),
                    operator,
                    posting_date,
                    Some(count_doc_id),
                    Some(line.line_no),
                    remark,
                )
                .await?;

            // 5. 回写明细 transaction_id
            let updated = sqlx::query(
                r#"
                UPDATE wms.wms_inventory_count_d
                SET
                    transaction_id = $3,
                    status = 'POSTED'
                WHERE count_doc_id = $1
                  AND line_no = $2
                "#,
            )
                .bind(count_doc_id)
                .bind(line.line_no)
                .bind(&transaction_id)
                .execute(&mut *tx)
                .await
                .map_err(InventoryCountApplicationError::database)?;

            if updated.rows_affected() == 0 {
                return Err(InventoryCountApplicationError::CountLineNotFound);
            }

            line.mark_posted(transaction_id.clone());

            transactions.push(InventoryCountPostedTransaction {
                line_no: line.line_no,
                transaction_id,
                movement_type: movement_type.to_string(),
                material_id: line.material_id.clone(),
                quantity: posting_qty,
                from_bin: from_bin.map(str::to_string),
                to_bin: to_bin.map(str::to_string),
                batch_number: line.batch_number.clone(),
            });
        }

        // 6. 领域层标记 POSTED
        count.mark_posted(operator.to_string())?;

        let posted_at = count.posted_at.unwrap_or_else(Utc::now);

        // 7. 更新盘点单头 posted 信息和状态
        let updated = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_h
            SET
                status = 'POSTED',
                posted_by = $2,
                posted_at = $3,
                remark = COALESCE($4, remark)
            WHERE count_doc_id = $1
            "#,
        )
            .bind(count_doc_id)
            .bind(operator)
            .bind(posted_at)
            .bind(remark)
            .execute(&mut *tx)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if updated.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountNotFound);
        }

        // 8. 提交事务
        tx.commit()
            .await
            .map_err(InventoryCountApplicationError::database)?;

        Ok(InventoryCountPostingResult {
            count_doc_id: count_doc_id.to_string(),
            status: InventoryCountStatus::Posted,
            transactions,
            // 不在事务内刷新物化视图，只告诉前端报表已过期
            reports_stale: true,
        })
    }
    /// MVP 只更新状态。
    /// 如果表里有 cancelled_by / cancelled_at，可以继续补字段。
    async fn cancel(
        &self,
        count_doc_id: &str,
        _cancelled_by: &str,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_h
            SET
                status = 'CANCELLED',
                remark = COALESCE($2, remark)
            WHERE count_doc_id = $1
            "#,
        )
            .bind(count_doc_id)
            .bind(remark)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if result.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountNotFound);
        }

        Ok(())
    }
    /// 检查同一范围是否存在未关闭盘点单
    ///
    /// 未关闭状态包括：
    /// DRAFT / COUNTING / SUBMITTED / APPROVED / POSTED
    ///
    /// CLOSED / CANCELLED 不算未关闭。
    async fn exists_open_count_for_scope(
        &self,
        scope: &InventoryCountScopeFilter,
    ) -> Result<bool, InventoryCountApplicationError> {
        let count_scope = count_scope_to_db(&scope.count_scope);

        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM wms.wms_inventory_count_h
                WHERE count_scope = $1
                  AND status NOT IN ('CLOSED', 'CANCELLED')
                  AND ($2::text IS NULL OR zone_code = $2)
                  AND ($3::text IS NULL OR bin_code = $3)
                  AND ($4::text IS NULL OR material_id = $4)
                  AND ($5::text IS NULL OR batch_number = $5)
            )
            "#,
        )
            .bind(count_scope)
            .bind(&scope.zone_code)
            .bind(&scope.bin_code)
            .bind(&scope.material_id)
            .bind(&scope.batch_number)
            .fetch_one(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        Ok(exists)
    }

    /// 盘盈 701 过账
    ///
    /// 下一段替换为真实 SQL 调用。
    async fn post_gain_701(
        &self,
        _line: &InventoryCountLine,
        _operator: &str,
        _posting_date: DateTime<Utc>,
        _remark: Option<&str>,
    ) -> Result<InventoryCountPostedTransaction, InventoryCountApplicationError> {
        Err(InventoryCountApplicationError::DifferencePostFailed(
            "post_gain_701 尚未实现".to_string(),
        ))
    }

    /// 盘亏 702 过账
    ///
    /// 下一段替换为真实 SQL 调用。
    async fn post_loss_702(
        &self,
        _line: &InventoryCountLine,
        _operator: &str,
        _posting_date: DateTime<Utc>,
        _remark: Option<&str>,
    ) -> Result<InventoryCountPostedTransaction, InventoryCountApplicationError> {
        Err(InventoryCountApplicationError::DifferencePostFailed(
            "post_loss_702 尚未实现".to_string(),
        ))
    }
    /// 更新审核信息
    async fn update_approved_info(
        &self,
        count_doc_id: &str,
        approved_by: &str,
        approved_at: DateTime<Utc>,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_h
            SET
                approved_by = $2,
                approved_at = $3,
                remark = COALESCE($4, remark)
            WHERE count_doc_id = $1
            "#,
        )
            .bind(count_doc_id)
            .bind(approved_by)
            .bind(approved_at)
            .bind(remark)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if result.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountNotFound);
        }

        Ok(())
    }
    /// 更新单行实盘数量
    /// 回写某一行的库存事务 ID
    async fn update_line_transaction_id(
        &self,
        count_doc_id: &str,
        line_no: i32,
        transaction_id: &str,
    ) -> Result<(), InventoryCountApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_d
            SET
                transaction_id = $3,
                status = 'POSTED'
            WHERE count_doc_id = $1
              AND line_no = $2
            "#,
        )
            .bind(count_doc_id)
            .bind(line_no)
            .bind(transaction_id)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if result.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountLineNotFound);
        }

        Ok(())
    }
    /// 批量更新实盘数量
    /// 更新过账信息
    async fn update_posted_info(
        &self,
        count_doc_id: &str,
        posted_by: &str,
        posted_at: DateTime<Utc>,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_h
            SET
                posted_by = $2,
                posted_at = $3,
                remark = COALESCE($4, remark)
            WHERE count_doc_id = $1
            "#,
        )
            .bind(count_doc_id)
            .bind(posted_by)
            .bind(posted_at)
            .bind(remark)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if result.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountNotFound);
        }

        Ok(())
    }

    /// 关闭盘点单
    ///
    /// 模板字段里有 closed_at。
    /// 如果你的表里有 closed_by，可以在 SQL 中追加 closed_by = $3。
    async fn close(
        &self,
        count_doc_id: &str,
        _closed_by: &str,
        closed_at: DateTime<Utc>,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_h
            SET
                status = 'CLOSED',
                closed_at = $2,
                remark = COALESCE($3, remark)
            WHERE count_doc_id = $1
            "#,
        )
            .bind(count_doc_id)
            .bind(closed_at)
            .bind(remark)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if result.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountNotFound);
        }

        Ok(())
    }
    /// 注意：
    /// service 层已经锁定 header 和 lines。
    /// 这里逐行更新，后续可以优化成 UNNEST 批量 SQL。
    async fn batch_update_lines(
        &self,
        count_doc_id: &str,
        lines: &[InventoryCountLine],
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError> {
        let mut updated_lines = Vec::with_capacity(lines.len());

        for line in lines {
            let movement_type = line
                .movement_type
                .as_ref()
                .map(InventoryCountMovementType::as_code)
                .map(str::to_string);

            let counted_qty = line
                .counted_qty
                .ok_or(InventoryCountApplicationError::LineNotCounted)?;

            let difference_qty = line
                .difference_qty
                .ok_or(InventoryCountApplicationError::LineNotCounted)?;

            let updated = self
                .update_line_counted_qty(
                    count_doc_id,
                    line.line_no,
                    counted_qty,
                    difference_qty,
                    movement_type,
                    line.difference_reason.clone(),
                    line.remark.clone(),
                )
                .await?;

            updated_lines.push(updated);
        }

        Ok(updated_lines)
    }
    /// 这里不重新计算差异。
    /// 更新盘点单状态
    ///
    /// operator 当前先不落库。
    /// 如果你的表里有 updated_by / updated_at，可以在这里一起维护。
    async fn update_status(
        &self,
        count_doc_id: &str,
        status: InventoryCountStatus,
        _operator: &str,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE wms.wms_inventory_count_h
            SET
                status = $2,
                remark = COALESCE($3, remark)
            WHERE count_doc_id = $1
            "#,
        )
            .bind(count_doc_id)
            .bind(count_status_to_db(&status))
            .bind(remark)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;

        if result.rows_affected() == 0 {
            return Err(InventoryCountApplicationError::CountNotFound);
        }

        Ok(())
    }
    /// 差异由 domain 层 InventoryCountLine::enter_counted_qty 计算好以后传进来。
    async fn update_line_counted_qty(
        &self,
        count_doc_id: &str,
        line_no: i32,
        counted_qty: Decimal,
        difference_qty: Decimal,
        movement_type: Option<String>,
        difference_reason: Option<String>,
        remark: Option<String>,
    ) -> Result<InventoryCountLine, InventoryCountApplicationError> {
        let row = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            UPDATE wms.wms_inventory_count_d
            SET
                counted_qty = $3,
                difference_qty = $4,
                movement_type = $5,
                difference_reason = $6,
                remark = $7,
                status = 'COUNTED'
            WHERE count_doc_id = $1
              AND line_no = $2
            RETURNING
                count_doc_id,
                line_no,
                material_id,
                bin_code,
                batch_number,
                quality_status,
                system_qty,
                counted_qty,
                difference_qty,
                difference_reason,
                movement_type,
                transaction_id,
                status,
                remark
            "#,
        )
            .bind(count_doc_id)
            .bind(line_no)
            .bind(counted_qty)
            .bind(difference_qty)
            .bind(movement_type)
            .bind(difference_reason)
            .bind(remark)
            .fetch_optional(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?
            .ok_or(InventoryCountApplicationError::CountLineNotFound)?;

        InventoryCountLine::try_from(row)
    }
    async fn next_count_doc_id(&self) -> Result<String, InventoryCountApplicationError> {
        let row = sqlx::query(
            r#"
            SELECT
                'COUNT-' || to_char(now(), 'YYYYMMDD') || '-' ||
                lpad(nextval('wms.seq_inventory_count_doc')::text, 6, '0') AS count_doc_id
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        let count_doc_id: String = row
            .try_get("count_doc_id")
            .map_err(InventoryCountApplicationError::database)?;

        Ok(count_doc_id)
    }

    /// 创建盘点单头
    async fn create_count(
        &self,
        count: &InventoryCount,
    ) -> Result<String, InventoryCountApplicationError> {
        sqlx::query(
            r#"
            INSERT INTO wms.wms_inventory_count_h (
                count_doc_id,
                count_type,
                count_scope,
                zone_code,
                bin_code,
                material_id,
                batch_number,
                status,
                created_by,
                created_at,
                remark
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11
            )
            "#,
        )
        .bind(&count.count_doc_id)
        .bind(count_type_to_db(&count.count_type))
        .bind(count_scope_to_db(&count.count_scope))
        .bind(&count.zone_code)
        .bind(&count.bin_code)
        .bind(&count.material_id)
        .bind(&count.batch_number)
        .bind(count_status_to_db(&count.status))
        .bind(&count.created_by)
        .bind(count.created_at)
        .bind(&count.remark)
        .execute(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        Ok(count.count_doc_id.clone())
    }

    /// 查询盘点单详情，包含明细
    async fn find_by_id(
        &self,
        count_doc_id: &str,
    ) -> Result<Option<InventoryCount>, InventoryCountApplicationError> {
        let header_row = sqlx::query_as::<_, InventoryCountHeaderRow>(
            r#"
            SELECT
                count_doc_id,
                count_type,
                count_scope,
                zone_code,
                bin_code,
                material_id,
                batch_number,
                status,
                created_by,
                approved_by,
                posted_by,
                created_at,
                approved_at,
                posted_at,
                closed_at,
                remark
            FROM wms.wms_inventory_count_h
            WHERE count_doc_id = $1
            "#,
        )
        .bind(count_doc_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        let Some(header_row) = header_row else {
            return Ok(None);
        };

        let mut count = InventoryCount::try_from(header_row)?;

        let line_rows = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            SELECT
                count_doc_id,
                line_no,
                material_id,
                bin_code,
                batch_number,
                quality_status,
                system_qty,
                counted_qty,
                difference_qty,
                difference_reason,
                movement_type,
                transaction_id,
                status,
                remark
            FROM wms.wms_inventory_count_d
            WHERE count_doc_id = $1
            ORDER BY line_no
            "#,
        )
        .bind(count_doc_id)
        .fetch_all(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        count.lines = line_rows
            .into_iter()
            .map(InventoryCountLine::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(count))
    }

    /// 查询盘点单列表
    ///
    /// 这里为了避免动态 SQL 过于复杂，使用：
    /// ($1 IS NULL OR column = $1)
    ///
    /// 后续如果性能敏感，可以改为 QueryBuilder 动态拼接 WHERE。
    async fn list(
        &self,
        input: &ListInventoryCountsInput,
    ) -> Result<Page<InventoryCountSummary>, InventoryCountApplicationError> {
        let page = input.page.page();
        let page_size = input.page.page_size();
        let offset = input.page.offset();

        let status = input.status.as_ref().map(count_status_to_db);
        let count_type = input.count_type.as_ref().map(count_type_to_db);
        let count_scope = input.count_scope.as_ref().map(count_scope_to_db);

        let rows = sqlx::query_as::<_, InventoryCountSummaryRow>(
            r#"
            SELECT
                h.count_doc_id,
                h.count_type,
                h.count_scope,
                h.zone_code,
                h.bin_code,
                h.material_id,
                h.batch_number,
                h.status,
                h.created_by,
                h.created_at,
                COUNT(d.line_no)::bigint AS line_count,
                COUNT(d.line_no) FILTER (
                    WHERE COALESCE(d.difference_qty, 0) <> 0
                )::bigint AS difference_line_count,
                h.remark
            FROM wms.wms_inventory_count_h h
            LEFT JOIN wms.wms_inventory_count_d d
                ON d.count_doc_id = h.count_doc_id
            WHERE
                ($1::text IS NULL OR h.status = $1)
                AND ($2::text IS NULL OR h.count_type = $2)
                AND ($3::text IS NULL OR h.count_scope = $3)
                AND ($4::text IS NULL OR h.zone_code = $4)
                AND ($5::text IS NULL OR h.bin_code = $5)
                AND ($6::text IS NULL OR h.material_id = $6)
                AND ($7::text IS NULL OR h.batch_number = $7)
                AND ($8::text IS NULL OR h.created_by = $8)
                AND ($9::timestamptz IS NULL OR h.created_at >= $9)
                AND ($10::timestamptz IS NULL OR h.created_at < $10)
            GROUP BY
                h.count_doc_id,
                h.count_type,
                h.count_scope,
                h.zone_code,
                h.bin_code,
                h.material_id,
                h.batch_number,
                h.status,
                h.created_by,
                h.created_at,
                h.remark
            ORDER BY h.created_at DESC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(status)
        .bind(count_type)
        .bind(count_scope)
        .bind(&input.zone_code)
        .bind(&input.bin_code)
        .bind(&input.material_id)
        .bind(&input.batch_number)
        .bind(&input.created_by)
        .bind(input.date_from)
        .bind(input.date_to)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM wms.wms_inventory_count_h h
            WHERE
                ($1::text IS NULL OR h.status = $1)
                AND ($2::text IS NULL OR h.count_type = $2)
                AND ($3::text IS NULL OR h.count_scope = $3)
                AND ($4::text IS NULL OR h.zone_code = $4)
                AND ($5::text IS NULL OR h.bin_code = $5)
                AND ($6::text IS NULL OR h.material_id = $6)
                AND ($7::text IS NULL OR h.batch_number = $7)
                AND ($8::text IS NULL OR h.created_by = $8)
                AND ($9::timestamptz IS NULL OR h.created_at >= $9)
                AND ($10::timestamptz IS NULL OR h.created_at < $10)
            "#,
        )
        .bind(status)
        .bind(count_type)
        .bind(count_scope)
        .bind(&input.zone_code)
        .bind(&input.bin_code)
        .bind(&input.material_id)
        .bind(&input.batch_number)
        .bind(&input.created_by)
        .bind(input.date_from)
        .bind(input.date_to)
        .fetch_one(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        let items = rows
            .into_iter()
            .map(InventoryCountSummary::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Page::new(items, page, page_size, total as u64))
    }

    /// 锁定盘点单头
    ///
    /// 注意：
    /// 这个方法本身只是 SQL 级 FOR UPDATE。
    /// 真正事务边界后面会通过 service 调用链或事务包装保证。
    async fn lock_header_for_update(
        &self,
        count_doc_id: &str,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let row = sqlx::query_as::<_, InventoryCountHeaderRow>(
            r#"
            SELECT
                count_doc_id,
                count_type,
                count_scope,
                zone_code,
                bin_code,
                material_id,
                batch_number,
                status,
                created_by,
                approved_by,
                posted_by,
                created_at,
                approved_at,
                posted_at,
                closed_at,
                remark
            FROM wms.wms_inventory_count_h
            WHERE count_doc_id = $1
            FOR UPDATE
            "#,
        )
        .bind(count_doc_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?
        .ok_or(InventoryCountApplicationError::CountNotFound)?;

        InventoryCount::try_from(row)
    }

    /// 锁定盘点明细
    async fn lock_lines_for_update(
        &self,
        count_doc_id: &str,
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError> {
        let rows = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            SELECT
                count_doc_id,
                line_no,
                material_id,
                bin_code,
                batch_number,
                quality_status,
                system_qty,
                counted_qty,
                difference_qty,
                difference_reason,
                movement_type,
                transaction_id,
                status,
                remark
            FROM wms.wms_inventory_count_d
            WHERE count_doc_id = $1
            ORDER BY line_no
            FOR UPDATE
            "#,
        )
        .bind(count_doc_id)
        .fetch_all(&self.pool)
        .await
        .map_err(InventoryCountApplicationError::database)?;

        rows.into_iter()
            .map(InventoryCountLine::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    /// 根据盘点范围从 wms.wms_bin_stock 生成明细快照
    async fn generate_lines_from_scope(
        &self,
        count_doc_id: &str,
        scope: &InventoryCountScopeFilter,
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError> {
        let rows = match scope.count_scope {
            InventoryCountScope::Bin => {
                sqlx::query_as::<_, GeneratedCountLineRow>(
                    r#"
                    SELECT
                        ROW_NUMBER() OVER (ORDER BY material_id, batch_number, quality_status)::int * 10 AS line_no,
                        material_id,
                        bin_code,
                        batch_number,
                        quality_status,
                        qty AS system_qty
                    FROM wms.wms_bin_stock
                    WHERE bin_code = $1
                      AND qty <> 0
                    ORDER BY material_id, batch_number, quality_status
                    "#,
                )
                .bind(scope.bin_code.as_deref())
                .fetch_all(&self.pool)
                .await
                .map_err(InventoryCountApplicationError::database)?
            }

            InventoryCountScope::Material => {
                sqlx::query_as::<_, GeneratedCountLineRow>(
                    r#"
                    SELECT
                        ROW_NUMBER() OVER (ORDER BY bin_code, batch_number, quality_status)::int * 10 AS line_no,
                        material_id,
                        bin_code,
                        batch_number,
                        quality_status,
                        qty AS system_qty
                    FROM wms.wms_bin_stock
                    WHERE material_id = $1
                      AND qty <> 0
                    ORDER BY bin_code, batch_number, quality_status
                    "#,
                )
                .bind(scope.material_id.as_deref())
                .fetch_all(&self.pool)
                .await
                .map_err(InventoryCountApplicationError::database)?
            }

            InventoryCountScope::Zone => {
                sqlx::query_as::<_, GeneratedCountLineRow>(
                    r#"
                    SELECT
                        ROW_NUMBER() OVER (ORDER BY bs.bin_code, bs.material_id, bs.batch_number, bs.quality_status)::int * 10 AS line_no,
                        bs.material_id,
                        bs.bin_code,
                        bs.batch_number,
                        bs.quality_status,
                        bs.qty AS system_qty
                    FROM wms.wms_bin_stock bs
                    INNER JOIN wms.wms_bins b
                        ON b.bin_code = bs.bin_code
                    WHERE b.zone_code = $1
                      AND bs.qty <> 0
                    ORDER BY bs.bin_code, bs.material_id, bs.batch_number, bs.quality_status
                    "#,
                )
                .bind(scope.zone_code.as_deref())
                .fetch_all(&self.pool)
                .await
                .map_err(InventoryCountApplicationError::database)?
            }

            // MVP 暂不支持 FULL / BATCH / CYCLE
            _ => {
                return Err(InventoryCountApplicationError::ScopeInvalid);
            }
        };

        Ok(rows
            .into_iter()
            .map(|row| {
                InventoryCountLine::from_stock_snapshot(
                    count_doc_id.to_string(),
                    row.line_no,
                    row.material_id,
                    row.bin_code,
                    row.batch_number,
                    row.quality_status,
                    row.system_qty,
                )
            })
            .collect())
    }

    /// 批量插入盘点明细
    async fn insert_lines(
        &self,
        count_doc_id: &str,
        lines: &[InventoryCountLine],
    ) -> Result<(), InventoryCountApplicationError> {
        for line in lines {
            sqlx::query(
                r#"
                INSERT INTO wms.wms_inventory_count_d (
                    count_doc_id,
                    line_no,
                    material_id,
                    bin_code,
                    batch_number,
                    quality_status,
                    system_qty,
                    counted_qty,
                    difference_qty,
                    difference_reason,
                    movement_type,
                    transaction_id,
                    status,
                    remark
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6,
                    $7, $8, $9, $10, $11, $12,
                    $13, $14
                )
                "#,
            )
            .bind(count_doc_id)
            .bind(line.line_no)
            .bind(&line.material_id)
            .bind(&line.bin_code)
            .bind(&line.batch_number)
            .bind(&line.quality_status)
            .bind(line.system_qty)
            .bind(line.counted_qty)
            .bind(line.difference_qty)
            .bind(&line.difference_reason)
            .bind(line.movement_type.as_ref().map(InventoryCountMovementType::as_code))
            .bind(&line.transaction_id)
            .bind(line_status_to_db(&line.status))
            .bind(&line.remark)
            .execute(&self.pool)
            .await
            .map_err(InventoryCountApplicationError::database)?;
        }

        Ok(())
    }

    // 剩下的方法下一段继续实现：
    // update_line_counted_qty
    // batch_update_lines
    // update_status
    // update_approved_info
    // post_gain_701
    /// 盘盈 701 过账
    ///
    /// 业务规则：
    /// - movement_type = '701'
    /// - quantity = difference_qty
    /// - from_bin = NULL
    /// - to_bin = line.bin_code
    ///
    /// 注意：
    /// wms.post_inventory_transaction() 是库存最终过账函数。
    /// 它会负责更新库存、批次、货位和库存事务。
    async fn post_gain_701(
        &self,
        line: &InventoryCountLine,
        operator: &str,
        posting_date: DateTime<Utc>,
        remark: Option<&str>,
    ) -> Result<InventoryCountPostedTransaction, InventoryCountApplicationError> {
        let qty = line.posting_qty();

        if qty <= Decimal::ZERO {
            return Err(InventoryCountApplicationError::DifferencePostFailed(
                format!("盘盈行数量必须大于 0，line_no={}", line.line_no),
            ));
        }

        let transaction_id = self
            .call_post_inventory_transaction(
                "701",
                &line.material_id,
                line.batch_number.as_deref(),
                qty,
                None,
                Some(&line.bin_code),
                line.quality_status.as_deref(),
                operator,
                posting_date,
                Some(&line.count_doc_id),
                Some(line.line_no),
                remark,
            )
            .await?;

        Ok(InventoryCountPostedTransaction {
            line_no: line.line_no,
            transaction_id,
            movement_type: "701".to_string(),
            material_id: line.material_id.clone(),
            quantity: qty,
            from_bin: None,
            to_bin: Some(line.bin_code.clone()),
            batch_number: line.batch_number.clone(),
        })
    }
    // post_loss_702
    /// 盘亏 702 过账
    ///
    /// 业务规则：
    /// - movement_type = '702'
    /// - quantity = ABS(difference_qty)
    /// - from_bin = line.bin_code
    /// - to_bin = NULL
    ///
    /// 质量控制提醒：
    /// Phase 8 模板要求出库类动作最终由 wms.post_inventory_transaction()
    /// 做数据库强校验。盘亏 702 属于出库类动作，也应该继承批次质量拦截规则。
    async fn post_loss_702(
        &self,
        line: &InventoryCountLine,
        operator: &str,
        posting_date: DateTime<Utc>,
        remark: Option<&str>,
    ) -> Result<InventoryCountPostedTransaction, InventoryCountApplicationError> {
        let qty = line.posting_qty();

        if qty <= Decimal::ZERO {
            return Err(InventoryCountApplicationError::DifferencePostFailed(
                format!("盘亏行数量必须大于 0，line_no={}", line.line_no),
            ));
        }

        let transaction_id = self
            .call_post_inventory_transaction(
                "702",
                &line.material_id,
                line.batch_number.as_deref(),
                qty,
                Some(&line.bin_code),
                None,
                line.quality_status.as_deref(),
                operator,
                posting_date,
                Some(&line.count_doc_id),
                Some(line.line_no),
                remark,
            )
            .await?;

        Ok(InventoryCountPostedTransaction {
            line_no: line.line_no,
            transaction_id,
            movement_type: "702".to_string(),
            material_id: line.material_id.clone(),
            quantity: qty,
            from_bin: Some(line.bin_code.clone()),
            to_bin: None,
            batch_number: line.batch_number.clone(),
        })
    }
    // update_line_transaction_id
    // update_posted_info
    // close
    // cancel
    // exists_open_count_for_scope
}

/* -------------------------------------------------------------------------- */
/*                             生成盘点明细 Row                                */
/* -------------------------------------------------------------------------- */

#[derive(Debug, sqlx::FromRow)]
struct GeneratedCountLineRow {
    pub line_no: i32,
    pub material_id: String,
    pub bin_code: String,
    pub batch_number: Option<String>,
    pub quality_status: Option<String>,
    pub system_qty: Decimal,
}

/* -------------------------------------------------------------------------- */
/*                           数据库字符串 <-> 枚举                             */
/* -------------------------------------------------------------------------- */

fn count_type_to_db(value: &InventoryCountType) -> &'static str {
    match value {
        InventoryCountType::Regular => "REGULAR",
        InventoryCountType::Cycle => "CYCLE",
        InventoryCountType::Adjustment => "ADJUSTMENT",
        InventoryCountType::YearEnd => "YEAR_END",
    }
}

fn parse_count_type(value: &str) -> Result<InventoryCountType, InventoryCountApplicationError> {
    match value {
        "REGULAR" => Ok(InventoryCountType::Regular),
        "CYCLE" => Ok(InventoryCountType::Cycle),
        "ADJUSTMENT" => Ok(InventoryCountType::Adjustment),
        "YEAR_END" => Ok(InventoryCountType::YearEnd),
        _ => Err(InventoryCountApplicationError::database(format!(
            "未知盘点类型: {value}"
        ))),
    }
}

fn count_scope_to_db(value: &InventoryCountScope) -> &'static str {
    match value {
        InventoryCountScope::Full => "FULL",
        InventoryCountScope::Zone => "ZONE",
        InventoryCountScope::Bin => "BIN",
        InventoryCountScope::Material => "MATERIAL",
        InventoryCountScope::Batch => "BATCH",
        InventoryCountScope::Cycle => "CYCLE",
    }
}

fn parse_count_scope(value: &str) -> Result<InventoryCountScope, InventoryCountApplicationError> {
    match value {
        "FULL" => Ok(InventoryCountScope::Full),
        "ZONE" => Ok(InventoryCountScope::Zone),
        "BIN" => Ok(InventoryCountScope::Bin),
        "MATERIAL" => Ok(InventoryCountScope::Material),
        "BATCH" => Ok(InventoryCountScope::Batch),
        "CYCLE" => Ok(InventoryCountScope::Cycle),
        _ => Err(InventoryCountApplicationError::database(format!(
            "未知盘点范围: {value}"
        ))),
    }
}

fn count_status_to_db(value: &InventoryCountStatus) -> &'static str {
    match value {
        InventoryCountStatus::Draft => "DRAFT",
        InventoryCountStatus::Counting => "COUNTING",
        InventoryCountStatus::Submitted => "SUBMITTED",
        InventoryCountStatus::Approved => "APPROVED",
        InventoryCountStatus::Posted => "POSTED",
        InventoryCountStatus::Closed => "CLOSED",
        InventoryCountStatus::Cancelled => "CANCELLED",
    }
}

fn parse_count_status(value: &str) -> Result<InventoryCountStatus, InventoryCountApplicationError> {
    match value {
        "DRAFT" => Ok(InventoryCountStatus::Draft),
        "COUNTING" => Ok(InventoryCountStatus::Counting),
        "SUBMITTED" => Ok(InventoryCountStatus::Submitted),
        "APPROVED" => Ok(InventoryCountStatus::Approved),
        "POSTED" => Ok(InventoryCountStatus::Posted),
        "CLOSED" => Ok(InventoryCountStatus::Closed),
        "CANCELLED" => Ok(InventoryCountStatus::Cancelled),
        _ => Err(InventoryCountApplicationError::database(format!(
            "未知盘点状态: {value}"
        ))),
    }
}

fn line_status_to_db(value: &InventoryCountLineStatus) -> &'static str {
    match value {
        InventoryCountLineStatus::Pending => "PENDING",
        InventoryCountLineStatus::Counted => "COUNTED",
        InventoryCountLineStatus::Posted => "POSTED",
    }
}

fn parse_line_status(value: &str) -> Result<InventoryCountLineStatus, InventoryCountApplicationError> {
    match value {
        "PENDING" => Ok(InventoryCountLineStatus::Pending),
        "COUNTED" => Ok(InventoryCountLineStatus::Counted),
        "POSTED" => Ok(InventoryCountLineStatus::Posted),
        _ => Err(InventoryCountApplicationError::database(format!(
            "未知盘点明细状态: {value}"
        ))),
    }
}

fn parse_movement_type_opt(
    value: Option<&str>,
) -> Result<Option<InventoryCountMovementType>, InventoryCountApplicationError> {
    match value {
        None => Ok(None),
        Some("701") => Ok(Some(InventoryCountMovementType::Gain701)),
        Some("702") => Ok(Some(InventoryCountMovementType::Loss702)),
        Some(other) => Err(InventoryCountApplicationError::database(format!(
            "未知盘点移动类型: {other}"
        ))),
    }
}