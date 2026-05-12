use crate::domain::{
    BatchNumber, BatchQuality, BatchQualityHistory, BatchQualityStatus, DefectCode,
    InspectionCharId, InspectionDecision, InspectionLot, InspectionLotId, InspectionLotStatus,
    InspectionLotType, InspectionResult, InspectionResultId, MaterialId, QualityNotification,
    QualityNotificationId, QualityNotificationSeverity, QualityNotificationStatus, QualityResult,
};
use async_trait::async_trait;
use cuba_shared::{Page, PageQuery};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// 质量模块 ID 生成器。
///
/// 具体实现可以是：
/// - 数据库序列
/// - ULID
/// - UUID
/// - 日期流水号
///
/// 示例：
/// - IL-20260504-000001
/// - IR-20260504-000001
/// - QN-20260504-000001
pub trait QualityIdGenerator: Send + Sync {
    fn next_inspection_lot_id(&self) -> InspectionLotId;

    fn next_inspection_result_id(&self) -> InspectionResultId;

    fn next_quality_notification_id(&self) -> QualityNotificationId;
}

// =============================================================================
// 检验批 Repository
// =============================================================================

/// 检验批查询条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionLotQuery {
    pub page: PageQuery,

    pub lot_type: Option<InspectionLotType>,
    pub status: Option<InspectionLotStatus>,
    pub material_id: Option<MaterialId>,
    pub batch_number: Option<BatchNumber>,
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
}

/// 检验批列表摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionLotSummary {
    pub id: InspectionLotId,
    pub lot_type: InspectionLotType,
    pub material_id: MaterialId,
    pub batch_number: BatchNumber,
    pub quantity: Decimal,
    pub sample_qty: Decimal,
    pub status: InspectionLotStatus,
    pub decision: Option<InspectionDecision>,
    pub created_at: OffsetDateTime,
}

/// 检验批仓储接口。
#[async_trait]
pub trait InspectionLotRepository: Send + Sync {
    /// 创建检验批。
    async fn create(&self, lot: &InspectionLot) -> QualityResult<InspectionLotId>;

    /// 按 ID 查询。
    async fn find_by_id(&self, lot_id: &InspectionLotId) -> QualityResult<Option<InspectionLot>>;

    /// 分页查询检验批。
    async fn list(&self, query: InspectionLotQuery) -> QualityResult<Page<InspectionLotSummary>>;

    /// 锁定检验批。
    ///
    /// PostgreSQL 实现中应该使用：
    /// SELECT ... FOR UPDATE
    async fn lock_by_id(&self, lot_id: &InspectionLotId) -> QualityResult<InspectionLot>;

    /// 更新检验批状态快照。
    async fn update(&self, lot: &InspectionLot) -> QualityResult<()>;

    /// 检查同一批次是否已有未关闭检验批。
    async fn exists_open_by_batch(&self, batch_number: &BatchNumber) -> QualityResult<bool>;
}

// =============================================================================
// 检验结果 Repository
// =============================================================================

/// 检验结果仓储接口。
#[async_trait]
pub trait InspectionResultRepository: Send + Sync {
    /// 创建单条检验结果。
    async fn create(&self, result: &InspectionResult) -> QualityResult<InspectionResultId>;

    /// 批量创建检验结果。
    async fn batch_create(
        &self,
        results: &[InspectionResult],
    ) -> QualityResult<Vec<InspectionResultId>>;

    /// 查询某个检验批下的所有结果。
    async fn find_by_lot_id(
        &self,
        lot_id: &InspectionLotId,
    ) -> QualityResult<Vec<InspectionResult>>;

    /// 按结果 ID 查询。
    async fn find_by_id(
        &self,
        result_id: &InspectionResultId,
    ) -> QualityResult<Option<InspectionResult>>;

    /// 更新检验结果。
    async fn update(&self, result: &InspectionResult) -> QualityResult<()>;

    /// 当前检验批是否已有检验结果。
    async fn has_any_result(&self, lot_id: &InspectionLotId) -> QualityResult<bool>;

    /// 当前检验批是否存在失败项。
    async fn has_failed_result(&self, lot_id: &InspectionLotId) -> QualityResult<bool>;
}

// =============================================================================
// 质量通知 Repository
// =============================================================================

/// 质量通知查询条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityNotificationQuery {
    pub page: PageQuery,

    pub status: Option<QualityNotificationStatus>,
    pub severity: Option<QualityNotificationSeverity>,
    pub material_id: Option<MaterialId>,
    pub batch_number: Option<BatchNumber>,
    pub owner: Option<String>,
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
}

/// 质量通知列表摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityNotificationSummary {
    pub id: QualityNotificationId,
    pub material_id: MaterialId,
    pub batch_number: BatchNumber,
    pub severity: QualityNotificationSeverity,
    pub status: QualityNotificationStatus,
    pub description: String,
    pub created_at: OffsetDateTime,
}

/// 质量通知仓储接口。
#[async_trait]
pub trait QualityNotificationRepository: Send + Sync {
    async fn create(
        &self,
        notification: &QualityNotification,
    ) -> QualityResult<QualityNotificationId>;

    async fn find_by_id(
        &self,
        notification_id: &QualityNotificationId,
    ) -> QualityResult<Option<QualityNotification>>;

    async fn list(
        &self,
        query: QualityNotificationQuery,
    ) -> QualityResult<Page<QualityNotificationSummary>>;

    async fn update(&self, notification: &QualityNotification) -> QualityResult<()>;
}

// =============================================================================
// 批次质量 Repository
// =============================================================================

/// 批次质量状态查询结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQualityStatusView {
    pub batch_number: BatchNumber,
    pub material_id: MaterialId,
    pub quality_status: BatchQualityStatus,
    pub current_qty: Decimal,
}

/// 批次历史查询条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHistoryQuery {
    pub page: PageQuery,
}

/// 批次质量仓储接口。
#[async_trait]
pub trait BatchQualityRepository: Send + Sync {
    /// 锁定批次质量状态。
    ///
    /// PostgreSQL 实现中应该使用：
    /// SELECT ... FROM wms.wms_batches WHERE batch_number = $1 FOR UPDATE
    async fn lock_batch_for_update(
        &self,
        batch_number: &BatchNumber,
    ) -> QualityResult<BatchQuality>;

    /// 更新批次质量状态。
    async fn update_quality_status(
        &self,
        batch_number: &BatchNumber,
        status: BatchQualityStatus,
    ) -> QualityResult<()>;

    /// 写入批次质量历史。
    async fn write_batch_history(&self, history: &BatchQualityHistory) -> QualityResult<()>;

    /// 查询批次质量状态。
    async fn get_batch_status(
        &self,
        batch_number: &BatchNumber,
    ) -> QualityResult<BatchQualityStatusView>;

    /// 查询批次历史。
    async fn list_batch_history(
        &self,
        batch_number: &BatchNumber,
        query: BatchHistoryQuery,
    ) -> QualityResult<Page<BatchQualityHistory>>;
}

// =============================================================================
// 质量主数据 Repository
// =============================================================================

/// 检验特性快照。
///
/// 来自 mdm.mdm_inspection_chars。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionCharSnapshot {
    pub char_id: InspectionCharId,
    pub char_code: String,
    pub char_name: String,
    pub lower_limit: Option<Decimal>,
    pub upper_limit: Option<Decimal>,
    pub unit: Option<String>,
    pub is_active: bool,
}

/// 不良代码快照。
///
/// 来自 mdm.mdm_defect_codes。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectCodeSnapshot {
    pub defect_code: DefectCode,
    pub description: String,
    pub is_active: bool,
}

/// 质量主数据仓储接口。
#[async_trait]
pub trait QualityMasterRepository: Send + Sync {
    /// 物料是否存在且启用。
    async fn material_exists_and_active(&self, material_id: &MaterialId) -> QualityResult<bool>;

    /// 查询检验特性。
    async fn find_inspection_char(
        &self,
        char_id: &InspectionCharId,
    ) -> QualityResult<Option<InspectionCharSnapshot>>;

    /// 查询不良代码。
    async fn find_defect_code(
        &self,
        defect_code: &DefectCode,
    ) -> QualityResult<Option<DefectCodeSnapshot>>;

    /// 按物料列出可用检验特性。
    async fn list_inspection_chars_for_material(
        &self,
        material_id: &MaterialId,
    ) -> QualityResult<Vec<InspectionCharSnapshot>>;
}
