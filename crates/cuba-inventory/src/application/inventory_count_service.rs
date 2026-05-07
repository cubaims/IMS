use crate::application::common::Page;
use crate::application::errors::InventoryCountApplicationError;
use crate::application::inventory_count_model::{
    ApproveInventoryCountInput, BatchUpdateInventoryCountLinesInput, CancelInventoryCountInput,
    CloseInventoryCountInput, CreateInventoryCountInput, GenerateInventoryCountLinesInput,
    GetInventoryCountInput, InventoryCountScopeFilter, ListInventoryCountsInput,
    PostInventoryCountInput, SubmitInventoryCountInput, UpdateInventoryCountLineInput,
};
use crate::application::inventory_count_repository::{
    InventoryCountRepository, InventoryCountSummary,
};
use crate::domain::{
    InventoryCount, InventoryCountLine, InventoryCountMovementType, InventoryCountStatus,
};
use std::sync::Arc;
use time::OffsetDateTime;

/// 盘点应用服务
///
/// 不读写分离：
/// 所有读、写、锁定、过账都只依赖一个 InventoryCountRepository。
pub struct InventoryCountService<R>
where
    R: InventoryCountRepository,
{
    repo: Arc<R>,
}

impl<R> InventoryCountService<R>
where
    R: InventoryCountRepository,
{
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// 创建盘点单
    ///
    /// 规则：
    /// 1. 盘点范围必须合法
    /// 2. 同一范围不能存在未关闭盘点单
    /// 3. 初始状态为 DRAFT
    pub async fn create_count(
        &self,
        input: CreateInventoryCountInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let count_doc_id = self.repo.next_count_doc_id().await?;

        let count = InventoryCount::new(
            count_doc_id.clone(),
            input.count_type,
            input.count_scope,
            input.zone_code,
            input.bin_code,
            input.material_id,
            input.batch_number,
            input.operator,
            input.remark,
        )?;

        let scope_filter = Self::scope_filter_from_count(&count);

        let exists_open = self.repo.exists_open_count_for_scope(&scope_filter).await?;

        if exists_open {
            return Err(InventoryCountApplicationError::DuplicatedScope);
        }

        self.repo.create_count(&count).await?;

        Ok(count)
    }

    /// 生成盘点明细
    ///
    /// 规则：
    /// 1. 只有 DRAFT 状态允许生成
    /// 2. 从 wms.wms_bin_stock 读取账面库存快照
    /// 3. 写入 wms.wms_inventory_count_d
    /// 4. 状态改为 COUNTING
    ///
    /// 注意：
    /// 下一部分 SQLx 实现时，insert_lines + update_status 必须放在同一个数据库事务中。
    pub async fn generate_lines(
        &self,
        input: GenerateInventoryCountLinesInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let mut count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        if !count.status.can_generate_lines() {
            return Err(InventoryCountApplicationError::StatusInvalid);
        }

        let scope_filter = Self::scope_filter_from_count(&count);

        let lines = self
            .repo
            .generate_lines_from_scope(&input.count_doc_id, &scope_filter)
            .await?;

        if lines.is_empty() {
            return Err(InventoryCountApplicationError::NoLines);
        }

        // 领域对象负责状态流转校验
        count.mark_counting(lines.clone())?;

        self.repo.insert_lines(&input.count_doc_id, &lines).await?;

        self.repo
            .update_status(
                &input.count_doc_id,
                InventoryCountStatus::Counting,
                &input.operator,
                None,
            )
            .await?;

        self.get_count(GetInventoryCountInput {
            count_doc_id: input.count_doc_id,
        })
        .await
    }

    /// 查询盘点单详情
    pub async fn get_count(
        &self,
        input: GetInventoryCountInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        self.repo
            .find_by_id(&input.count_doc_id)
            .await?
            .ok_or(InventoryCountApplicationError::CountNotFound)
    }

    /// 查询盘点单列表
    pub async fn list_counts(
        &self,
        input: ListInventoryCountsInput,
    ) -> Result<Page<InventoryCountSummary>, InventoryCountApplicationError> {
        self.repo.list(&input).await
    }

    /// 录入单行实盘数量
    ///
    /// 规则：
    /// 1. 只有 COUNTING 状态允许录入
    /// 2. counted_qty 不能小于 0
    /// 3. 自动计算 difference_qty = counted_qty - system_qty
    /// 4. difference_qty > 0 自动生成 701
    /// 5. difference_qty < 0 自动生成 702
    /// 6. difference_qty = 0 不生成 movement_type
    pub async fn update_line(
        &self,
        input: UpdateInventoryCountLineInput,
    ) -> Result<InventoryCountLine, InventoryCountApplicationError> {
        let count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        if !count.status.can_update_counted_qty() {
            return Err(InventoryCountApplicationError::StatusInvalid);
        }

        let mut lines = self.repo.lock_lines_for_update(&input.count_doc_id).await?;

        let line = lines
            .iter_mut()
            .find(|line| line.line_no == input.line_no)
            .ok_or(InventoryCountApplicationError::CountLineNotFound)?;

        // 领域对象负责差异计算和 701 / 702 判断
        line.enter_counted_qty(input.counted_qty, input.difference_reason, input.remark)?;

        let difference_qty = line
            .difference_qty
            .ok_or(InventoryCountApplicationError::LineNotCounted)?;

        let movement_type = Self::movement_type_code(line);

        self.repo
            .update_line_counted_qty(
                &input.count_doc_id,
                input.line_no,
                input.counted_qty,
                difference_qty,
                movement_type,
                line.difference_reason.clone(),
                line.remark.clone(),
            )
            .await
    }

    /// 批量录入实盘数量
    ///
    /// 这里仍然不做读写分离，只是一次性锁定 Header 和 Lines。
    ///
    /// 注意：
    /// 下一部分 SQLx 实现时，这个方法内部所有更新必须放在同一个事务中。
    pub async fn batch_update_lines(
        &self,
        input: BatchUpdateInventoryCountLinesInput,
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError> {
        let count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        if !count.status.can_update_counted_qty() {
            return Err(InventoryCountApplicationError::StatusInvalid);
        }

        let mut existing_lines = self.repo.lock_lines_for_update(&input.count_doc_id).await?;

        for item in input.lines {
            let line = existing_lines
                .iter_mut()
                .find(|line| line.line_no == item.line_no)
                .ok_or(InventoryCountApplicationError::CountLineNotFound)?;

            // 每一行都走同一个领域方法，避免批量逻辑绕过规则
            line.enter_counted_qty(item.counted_qty, item.difference_reason, item.remark)?;
        }

        self.repo
            .batch_update_lines(&input.count_doc_id, &existing_lines)
            .await
    }

    /// 提交盘点单
    ///
    /// 规则：
    /// 1. 只有 COUNTING 状态允许提交
    /// 2. 所有行必须录入 counted_qty
    /// 3. 存在差异的行必须填写 difference_reason
    /// 4. 提交后状态变为 SUBMITTED
    pub async fn submit(
        &self,
        input: SubmitInventoryCountInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let mut count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        let lines = self.repo.lock_lines_for_update(&input.count_doc_id).await?;

        if lines.is_empty() {
            return Err(InventoryCountApplicationError::NoLines);
        }

        count.lines = lines;

        // 领域对象负责：
        // - 状态校验
        // - 是否全部录入
        // - 差异原因是否必填
        count.submit()?;

        self.repo
            .update_status(
                &input.count_doc_id,
                InventoryCountStatus::Submitted,
                &input.operator,
                input.remark.as_deref(),
            )
            .await?;

        self.get_count(GetInventoryCountInput {
            count_doc_id: input.count_doc_id,
        })
        .await
    }

    /// 从盘点单头构造范围过滤条件
    fn scope_filter_from_count(count: &InventoryCount) -> InventoryCountScopeFilter {
        InventoryCountScopeFilter {
            count_scope: count.count_scope.clone(),
            zone_code: count.zone_code.clone(),
            bin_code: count.bin_code.clone(),
            material_id: count.material_id.clone(),
            batch_number: count.batch_number.clone(),
        }
    }

    /// 把领域枚举转换成数据库移动类型编码
    fn movement_type_code(line: &InventoryCountLine) -> Option<String> {
        line.movement_type
            .as_ref()
            .map(InventoryCountMovementType::as_code)
            .map(str::to_string)
    }

    /// 审核盘点单
    ///
    /// 规则：
    /// 1. 只有 SUBMITTED 状态允许审核
    /// 2. approved = true  时，状态变为 APPROVED
    /// 3. approved = false 时，退回 COUNTING，可重新录入实盘
    pub async fn approve(
        &self,
        input: crate::application::inventory_count_model::ApproveInventoryCountInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let mut count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        if !count.status.can_approve() {
            return Err(InventoryCountApplicationError::StatusInvalid);
        }

        if input.approved {
            // 领域对象负责审核状态流转
            count.approve(input.operator.clone())?;

            self.repo
                .update_approved_info(
                    &input.count_doc_id,
                    &input.operator,
                    count.approved_at.unwrap_or_else(OffsetDateTime::now_utc),
                    input.remark.as_deref(),
                )
                .await?;

            self.repo
                .update_status(
                    &input.count_doc_id,
                    InventoryCountStatus::Approved,
                    &input.operator,
                    input.remark.as_deref(),
                )
                .await?;
        } else {
            // 审核退回：SUBMITTED -> COUNTING
            count.reject_to_recount()?;

            self.repo
                .update_status(
                    &input.count_doc_id,
                    InventoryCountStatus::Counting,
                    &input.operator,
                    input.remark.as_deref(),
                )
                .await?;
        }

        self.get_count(GetInventoryCountInput {
            count_doc_id: input.count_doc_id,
        })
        .await
    }

    /// 盘点过账
    ///
    /// 事务边界放在 Repository 实现中：
    /// 1. 锁定盘点单头
    /// 2. 锁定盘点明细
    /// 3. 校验状态 = APPROVED
    /// 4. 差异行逐行调用 701 / 702
    /// 5. 回写 transaction_id
    /// 6. 更新盘点单状态 = POSTED
    /// 7. COMMIT
    pub async fn post(
        &self,
        input: crate::application::inventory_count_model::PostInventoryCountInput,
    ) -> Result<crate::domain::InventoryCountPostingResult, InventoryCountApplicationError> {
        self.repo
            .post_count_transactional(
                &input.count_doc_id,
                &input.operator,
                input.posting_date,
                input.remark.as_deref(),
            )
            .await
    }

    /// 关闭盘点单
    ///
    /// 规则：
    /// 1. 只有 POSTED 状态允许关闭
    /// 2. 关闭后只读，不允许继续修改
    pub async fn close(
        &self,
        input: crate::application::inventory_count_model::CloseInventoryCountInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let mut count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        // 领域对象负责 POSTED -> CLOSED 校验
        count.close()?;

        let closed_at = count.closed_at.unwrap_or_else(OffsetDateTime::now_utc);

        self.repo
            .close(
                &input.count_doc_id,
                &input.operator,
                closed_at,
                input.remark.as_deref(),
            )
            .await?;

        self.get_count(GetInventoryCountInput {
            count_doc_id: input.count_doc_id,
        })
        .await
    }

    /// 取消盘点单
    ///
    /// 规则：
    /// 1. CLOSED / CANCELLED 不能再取消
    /// 2. 已 POSTED 的盘点单通常不允许取消
    ///    因为库存事务已经生成，如需处理应走冲销或调整流程，MVP 先禁止。
    pub async fn cancel(
        &self,
        input: crate::application::inventory_count_model::CancelInventoryCountInput,
    ) -> Result<InventoryCount, InventoryCountApplicationError> {
        let mut count = self
            .repo
            .lock_header_for_update(&input.count_doc_id)
            .await?;

        if matches!(count.status, InventoryCountStatus::Posted) {
            return Err(InventoryCountApplicationError::AlreadyPosted);
        }

        // 领域对象负责 DRAFT / COUNTING / SUBMITTED / APPROVED -> CANCELLED
        count.cancel()?;

        self.repo
            .cancel(
                &input.count_doc_id,
                &input.operator,
                input.remark.as_deref(),
            )
            .await?;

        self.get_count(GetInventoryCountInput {
            count_doc_id: input.count_doc_id,
        })
        .await
    }
}
