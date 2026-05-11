use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;

use crate::application::common::Page;
use crate::application::errors::InventoryCountApplicationError;
use crate::application::inventory_count_model::{
    BatchUpdateInventoryCountLineItem, InventoryCountScopeFilter, ListInventoryCountsInput,
};
use crate::domain::{
    InventoryCount, InventoryCountLine, InventoryCountPostedTransaction,
    InventoryCountPostingResult, InventoryCountScope, InventoryCountStatus, InventoryCountType,
};

/// 盘点单列表摘要
#[derive(Debug, Clone, Serialize)] // ← 必须加上 Serialize
pub struct InventoryCountSummary {
    pub count_doc_id: String,
    pub count_type: InventoryCountType,
    pub count_scope: InventoryCountScope,

    pub zone_code: Option<String>,
    pub bin_code: Option<String>,
    pub material_id: Option<String>,
    pub batch_number: Option<String>,

    pub status: InventoryCountStatus,
    pub created_by: String,
    pub created_at: OffsetDateTime,

    pub line_count: i64,
    pub difference_line_count: i64,
    pub remark: Option<String>,
}

/// 不做读写分离：
///
/// 所有盘点相关数据库访问都放在一个 Repository 里。
/// 包括：
/// - 创建
/// - 查询
/// - 锁定
/// - 更新状态
/// - 录入实盘
/// - 盘点过账
/// - 关闭 / 取消
#[async_trait]
pub trait InventoryCountRepository: Send + Sync {
    /// 以下非事务方法是 PostgreSQL adapter 内部复用的低层 helper 契约。
    ///
    /// Application service 不应直接编排这些方法来完成命令；新增或修改盘点命令
    /// 必须优先使用后面的 `*_transactional` 方法，确保状态流转、库存过账和
    /// transaction_id 回写处于同一个数据库事务边界。

    /// 生成盘点单号
    ///
    /// 例如：COUNT-20260504-000001
    async fn next_count_doc_id(&self) -> Result<String, InventoryCountApplicationError>;

    /// 创建盘点单头
    async fn create_count(
        &self,
        count: &InventoryCount,
    ) -> Result<String, InventoryCountApplicationError>;

    /// 查询盘点单详情，包含明细
    async fn find_by_id(
        &self,
        count_doc_id: &str,
    ) -> Result<Option<InventoryCount>, InventoryCountApplicationError>;

    /// 查询盘点单列表
    async fn list(
        &self,
        input: &ListInventoryCountsInput,
    ) -> Result<Page<InventoryCountSummary>, InventoryCountApplicationError>;

    /// 锁定盘点单头
    ///
    /// PostgreSQL 实现：
    /// SELECT ... FROM wms.wms_inventory_count_h
    /// WHERE count_doc_id = $1
    /// FOR UPDATE
    async fn lock_header_for_update(
        &self,
        count_doc_id: &str,
    ) -> Result<InventoryCount, InventoryCountApplicationError>;

    /// 锁定盘点单明细
    ///
    /// PostgreSQL 实现：
    /// SELECT ... FROM wms.wms_inventory_count_d
    /// WHERE count_doc_id = $1
    /// ORDER BY line_no
    /// FOR UPDATE
    async fn lock_lines_for_update(
        &self,
        count_doc_id: &str,
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError>;

    /// 根据盘点范围从 wms.wms_bin_stock 生成账面库存快照
    async fn generate_lines_from_scope(
        &self,
        count_doc_id: &str,
        scope: &InventoryCountScopeFilter,
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError>;

    /// 批量插入盘点明细
    async fn insert_lines(
        &self,
        count_doc_id: &str,
        lines: &[InventoryCountLine],
    ) -> Result<(), InventoryCountApplicationError>;

    /// 更新单行实盘数量
    ///
    /// counted_qty / difference_qty / movement_type
    /// 由领域对象先计算好，Repository 只负责落库。
    async fn update_line_counted_qty(
        &self,
        count_doc_id: &str,
        line_no: i32,
        counted_qty: Decimal,
        difference_qty: Decimal,
        movement_type: Option<String>,
        difference_reason: Option<String>,
        remark: Option<String>,
    ) -> Result<InventoryCountLine, InventoryCountApplicationError>;

    /// 批量更新实盘数量
    async fn batch_update_lines(
        &self,
        count_doc_id: &str,
        lines: &[InventoryCountLine],
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError>;

    /// 更新盘点单状态
    async fn update_status(
        &self,
        count_doc_id: &str,
        status: InventoryCountStatus,
        operator: &str,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError>;

    /// 更新审核信息
    async fn update_approved_info(
        &self,
        count_doc_id: &str,
        approved_by: &str,
        approved_at: OffsetDateTime,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError>;

    /// 盘盈过账
    ///
    /// 内部调用：
    /// wms.post_inventory_transaction(..., movement_type = '701')
    async fn post_gain_701(
        &self,
        line: &InventoryCountLine,
        operator: &str,
        posting_date: OffsetDateTime,
        remark: Option<&str>,
    ) -> Result<InventoryCountPostedTransaction, InventoryCountApplicationError>;

    /// 盘亏过账
    ///
    /// 内部调用：
    /// wms.post_inventory_transaction(..., movement_type = '702')
    async fn post_loss_702(
        &self,
        line: &InventoryCountLine,
        operator: &str,
        posting_date: OffsetDateTime,
        remark: Option<&str>,
    ) -> Result<InventoryCountPostedTransaction, InventoryCountApplicationError>;

    /// 回写某一行的库存事务 ID
    async fn update_line_transaction_id(
        &self,
        count_doc_id: &str,
        line_no: i32,
        transaction_id: &str,
    ) -> Result<(), InventoryCountApplicationError>;

    /// 更新过账信息
    async fn update_posted_info(
        &self,
        count_doc_id: &str,
        posted_by: &str,
        posted_at: OffsetDateTime,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError>;

    /// 关闭盘点单
    async fn close(
        &self,
        count_doc_id: &str,
        closed_by: &str,
        closed_at: OffsetDateTime,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError>;

    /// 取消盘点单
    async fn cancel(
        &self,
        count_doc_id: &str,
        cancelled_by: &str,
        remark: Option<&str>,
    ) -> Result<(), InventoryCountApplicationError>;

    /// 检查同一范围是否存在未关闭盘点单
    async fn exists_open_count_for_scope(
        &self,
        scope: &InventoryCountScopeFilter,
    ) -> Result<bool, InventoryCountApplicationError>;

    /// 创建盘点单事务方法。
    ///
    /// 这个方法内部必须完成：
    /// 1. BEGIN
    /// 2. 检查同一范围是否存在未关闭盘点单
    /// 3. 插入盘点单头
    /// 4. COMMIT
    async fn create_count_transactional(
        &self,
        count: &InventoryCount,
        scope: &InventoryCountScopeFilter,
    ) -> Result<String, InventoryCountApplicationError>;

    /// 生成盘点明细事务方法。
    ///
    /// 这个方法内部必须完成：
    /// 1. BEGIN
    /// 2. 锁定盘点单头
    /// 3. 校验状态 = DRAFT
    /// 4. 从 wms.wms_bin_stock 生成账面库存快照
    /// 5. 插入盘点明细
    /// 6. 更新状态 = COUNTING
    /// 7. COMMIT
    async fn generate_lines_transactional(
        &self,
        count_doc_id: &str,
        operator: &str,
    ) -> Result<InventoryCount, InventoryCountApplicationError>;

    /// 录入单行实盘数量事务方法。
    async fn update_line_transactional(
        &self,
        count_doc_id: &str,
        line_no: i32,
        counted_qty: Decimal,
        difference_reason: Option<String>,
        remark: Option<String>,
        operator: &str,
    ) -> Result<InventoryCountLine, InventoryCountApplicationError>;

    /// 批量录入实盘数量事务方法。
    async fn batch_update_lines_transactional(
        &self,
        count_doc_id: &str,
        updates: &[BatchUpdateInventoryCountLineItem],
        operator: &str,
    ) -> Result<Vec<InventoryCountLine>, InventoryCountApplicationError>;

    /// 提交盘点单事务方法。
    async fn submit_count_transactional(
        &self,
        count_doc_id: &str,
        operator: &str,
        remark: Option<&str>,
    ) -> Result<InventoryCount, InventoryCountApplicationError>;

    /// 审核盘点单事务方法。
    async fn approve_count_transactional(
        &self,
        count_doc_id: &str,
        approved: bool,
        operator: &str,
        remark: Option<&str>,
    ) -> Result<InventoryCount, InventoryCountApplicationError>;

    /// 关闭盘点单事务方法。
    async fn close_count_transactional(
        &self,
        count_doc_id: &str,
        operator: &str,
        remark: Option<&str>,
    ) -> Result<InventoryCount, InventoryCountApplicationError>;

    /// 取消盘点单事务方法。
    async fn cancel_count_transactional(
        &self,
        count_doc_id: &str,
        operator: &str,
        remark: Option<&str>,
    ) -> Result<InventoryCount, InventoryCountApplicationError>;

    /// 盘点过账事务方法
    ///
    /// 不做读写分离：
    /// 仍然放在同一个 InventoryCountRepository 里。
    ///
    /// 这个方法内部必须完成：
    /// 1. BEGIN
    /// 2. 锁定盘点单头
    /// 3. 锁定盘点明细
    /// 4. 校验状态 = APPROVED
    /// 5. 差异行逐行调用 701 / 702
    /// 6. 回写 transaction_id
    /// 7. 更新盘点单状态 = POSTED
    /// 8. COMMIT
    async fn post_count_transactional(
        &self,
        count_doc_id: &str,
        operator: &str,
        posting_date: OffsetDateTime,
        remark: Option<&str>,
    ) -> Result<InventoryCountPostingResult, InventoryCountApplicationError>;
}
