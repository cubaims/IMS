use crate::application::ports::{
    BatchQualityRepository, InspectionLotRepository, InspectionResultRepository,
    QualityIdGenerator, QualityMasterRepository, QualityNotificationRepository,
};
use crate::domain::{
    BatchNumber, BatchQualityAction, BatchQualityHistory, BatchQualityStatus, CreateInspectionLot,
    CreateInspectionResult, CreateQualityNotification, DefectCode, InspectionCharId,
    InspectionDecision, InspectionLot, InspectionLotId, InspectionLotType, InspectionResult,
    InspectionResultId, InspectionResultStatus, MaterialId, Operator, QualityError,
    QualityNotification, QualityNotificationId, QualityNotificationSeverity, QualityResult,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// =============================================================================
// 创建检验批
// =============================================================================

/// 创建检验批命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionLotCommand {
    pub lot_type: InspectionLotType,
    pub material_id: MaterialId,
    pub batch_number: BatchNumber,
    pub source_transaction_id: Option<String>,
    pub source_doc: Option<String>,
    pub quantity: Decimal,
    pub sample_qty: Decimal,
    pub created_by: Operator,
    pub remark: Option<String>,

    /// 创建检验批后是否把批次置为“待检”。
    ///
    /// 采购入库 / 生产入库自动生成检验批时，建议为 true。
    pub mark_batch_pending_inspection: bool,
}

/// 创建检验批结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionLotOutput {
    pub inspection_lot_id: InspectionLotId,
    pub batch_number: BatchNumber,
    pub batch_status_changed: bool,
}

/// 创建检验批用例。
pub struct CreateInspectionLotUseCase<L, B, M, G>
where
    L: InspectionLotRepository,
    B: BatchQualityRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub lot_repo: L,
    pub batch_repo: B,
    pub master_repo: M,
    pub id_generator: G,
}

impl<L, B, M, G> CreateInspectionLotUseCase<L, B, M, G>
where
    L: InspectionLotRepository,
    B: BatchQualityRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub fn new(lot_repo: L, batch_repo: B, master_repo: M, id_generator: G) -> Self {
        Self {
            lot_repo,
            batch_repo,
            master_repo,
            id_generator,
        }
    }

    /// 执行创建检验批。
    ///
    /// 事务建议：
    /// BEGIN
    ///   校验物料
    ///   锁定批次
    ///   检查是否已有未关闭检验批
    ///   创建 inspection_lot
    ///   可选更新批次为待检
    ///   写 batch_history
    /// COMMIT
    pub async fn execute(
        &self,
        command: CreateInspectionLotCommand,
    ) -> QualityResult<CreateInspectionLotOutput> {
        let now = OffsetDateTime::now_utc();

        let material_ok = self
            .master_repo
            .material_exists_and_active(&command.material_id)
            .await?;

        if !material_ok {
            return Err(QualityError::BusinessRuleViolation(
                "物料不存在或已停用".to_string(),
            ));
        }

        let mut batch = self
            .batch_repo
            .lock_batch_for_update(&command.batch_number)
            .await?;

        let exists_open_lot = self
            .lot_repo
            .exists_open_by_batch(&command.batch_number)
            .await?;

        if exists_open_lot {
            return Err(QualityError::InspectionLotAlreadyExists);
        }

        let inspection_lot_id = self.id_generator.next_inspection_lot_id();

        let lot = InspectionLot::create(CreateInspectionLot {
            id: inspection_lot_id.clone(),
            lot_type: command.lot_type,
            material_id: command.material_id.clone(),
            batch_number: command.batch_number.clone(),
            source_transaction_id: command.source_transaction_id.clone(),
            source_doc: command.source_doc.clone(),
            quantity: command.quantity,
            sample_qty: command.sample_qty,
            created_by: command.created_by.clone(),
            now,
            remark: command.remark.clone(),
        })?;

        self.lot_repo.create(&lot).await?;

        let mut batch_status_changed = false;

        if command.mark_batch_pending_inspection {
            if batch.status == BatchQualityStatus::Scrapped {
                return Err(QualityError::BatchAlreadyScrapped);
            }

            if batch.status != BatchQualityStatus::PendingInspection {
                let old_status = batch.status;
                batch.status = BatchQualityStatus::PendingInspection;

                self.batch_repo
                    .update_quality_status(
                        &command.batch_number,
                        BatchQualityStatus::PendingInspection,
                    )
                    .await?;

                self.batch_repo
                    .write_batch_history(&BatchQualityHistory {
                        batch_number: command.batch_number.clone(),
                        old_status: Some(old_status),
                        new_status: BatchQualityStatus::PendingInspection,
                        action: BatchQualityAction::MarkPendingInspection,
                        reason: "创建检验批后标记为待检".to_string(),
                        reference_doc: command.source_doc.clone(),
                        operator: command.created_by.clone(),
                        occurred_at: now,
                        remark: command.remark.clone(),
                    })
                    .await?;

                batch_status_changed = true;
            }
        }

        Ok(CreateInspectionLotOutput {
            inspection_lot_id,
            batch_number: command.batch_number,
            batch_status_changed,
        })
    }
}

// =============================================================================
// 录入检验结果
// =============================================================================

/// 录入检验结果命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddInspectionResultCommand {
    pub inspection_lot_id: InspectionLotId,
    pub char_id: InspectionCharId,
    pub measured_value: Option<Decimal>,
    pub qualitative_result: Option<InspectionResultStatus>,
    pub defect_code: Option<DefectCode>,
    pub defect_qty: Decimal,
    pub inspector: Operator,
    pub remark: Option<String>,
}

/// 录入检验结果输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddInspectionResultOutput {
    pub result_id: InspectionResultId,
    pub result_status: InspectionResultStatus,
}

/// 批量录入检验结果命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAddInspectionResultsCommand {
    pub inspection_lot_id: InspectionLotId,
    pub results: Vec<BatchAddInspectionResultItem>,
    pub inspector: Operator,
}

/// 批量录入检验结果明细。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAddInspectionResultItem {
    pub char_id: InspectionCharId,
    pub measured_value: Option<Decimal>,
    pub qualitative_result: Option<InspectionResultStatus>,
    pub defect_code: Option<DefectCode>,
    pub defect_qty: Decimal,
    pub remark: Option<String>,
}

/// 批量录入检验结果输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAddInspectionResultsOutput {
    pub results: Vec<AddInspectionResultOutput>,
}

/// 录入检验结果用例。
pub struct AddInspectionResultUseCase<L, R, M, G>
where
    L: InspectionLotRepository,
    R: InspectionResultRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub lot_repo: L,
    pub result_repo: R,
    pub master_repo: M,
    pub id_generator: G,
}

impl<L, R, M, G> AddInspectionResultUseCase<L, R, M, G>
where
    L: InspectionLotRepository,
    R: InspectionResultRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub fn new(lot_repo: L, result_repo: R, master_repo: M, id_generator: G) -> Self {
        Self {
            lot_repo,
            result_repo,
            master_repo,
            id_generator,
        }
    }

    /// 执行录入检验结果。
    pub async fn execute(
        &self,
        command: AddInspectionResultCommand,
    ) -> QualityResult<AddInspectionResultOutput> {
        let now = OffsetDateTime::now_utc();

        let mut lot = self.lot_repo.lock_by_id(&command.inspection_lot_id).await?;

        if !lot.status.can_enter_result() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        let inspection_char = self
            .master_repo
            .find_inspection_char(&command.char_id)
            .await?
            .ok_or(QualityError::InspectionCharNotFound)?;

        if !inspection_char.is_active {
            return Err(QualityError::InspectionCharInactive);
        }

        if let Some(defect_code) = &command.defect_code {
            let defect = self
                .master_repo
                .find_defect_code(defect_code)
                .await?
                .ok_or(QualityError::DefectCodeNotFound)?;

            if !defect.is_active {
                return Err(QualityError::DefectCodeInactive);
            }
        }

        // 先生成临时领域 ID。真实入库 ID 由 PostgreSQL BIGSERIAL 返回。
        let temp_result_id = self.id_generator.next_inspection_result_id();

        let result = InspectionResult::create(CreateInspectionResult {
            id: temp_result_id,
            inspection_lot_id: command.inspection_lot_id.clone(),
            char_id: command.char_id,
            measured_value: command.measured_value,
            qualitative_result: command.qualitative_result,
            lower_limit: inspection_char.lower_limit,
            upper_limit: inspection_char.upper_limit,
            unit: inspection_char.unit,
            defect_code: command.defect_code,
            defect_qty: command.defect_qty,
            inspector: command.inspector.clone(),
            now,
            remark: command.remark,
        })?;

        let result_status = result.result_status;

        let result_id = self.result_repo.create(&result).await?;

        if lot.status.can_enter_result() {
            lot.mark_in_progress(command.inspector, now)?;
            self.lot_repo.update(&lot).await?;
        }

        Ok(AddInspectionResultOutput {
            result_id,
            result_status,
        })
    }
}

/// 批量录入检验结果用例。
pub struct BatchAddInspectionResultsUseCase<L, R, M, G>
where
    L: InspectionLotRepository,
    R: InspectionResultRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub lot_repo: L,
    pub result_repo: R,
    pub master_repo: M,
    pub id_generator: G,
}

impl<L, R, M, G> BatchAddInspectionResultsUseCase<L, R, M, G>
where
    L: InspectionLotRepository,
    R: InspectionResultRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub fn new(lot_repo: L, result_repo: R, master_repo: M, id_generator: G) -> Self {
        Self {
            lot_repo,
            result_repo,
            master_repo,
            id_generator,
        }
    }

    /// 执行批量录入检验结果。
    pub async fn execute(
        &self,
        command: BatchAddInspectionResultsCommand,
    ) -> QualityResult<BatchAddInspectionResultsOutput> {
        if command.results.is_empty() {
            return Err(QualityError::InspectionResultRequired);
        }

        let now = OffsetDateTime::now_utc();

        let mut lot = self.lot_repo.lock_by_id(&command.inspection_lot_id).await?;

        if !lot.status.can_enter_result() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        let mut results = Vec::with_capacity(command.results.len());
        let mut result_statuses = Vec::with_capacity(command.results.len());

        for item in command.results {
            let inspection_char = self
                .master_repo
                .find_inspection_char(&item.char_id)
                .await?
                .ok_or(QualityError::InspectionCharNotFound)?;

            if !inspection_char.is_active {
                return Err(QualityError::InspectionCharInactive);
            }

            if let Some(defect_code) = &item.defect_code {
                let defect = self
                    .master_repo
                    .find_defect_code(defect_code)
                    .await?
                    .ok_or(QualityError::DefectCodeNotFound)?;

                if !defect.is_active {
                    return Err(QualityError::DefectCodeInactive);
                }
            }

            let result = InspectionResult::create(CreateInspectionResult {
                id: self.id_generator.next_inspection_result_id(),
                inspection_lot_id: command.inspection_lot_id.clone(),
                char_id: item.char_id,
                measured_value: item.measured_value,
                qualitative_result: item.qualitative_result,
                lower_limit: inspection_char.lower_limit,
                upper_limit: inspection_char.upper_limit,
                unit: inspection_char.unit,
                defect_code: item.defect_code,
                defect_qty: item.defect_qty,
                inspector: command.inspector.clone(),
                now,
                remark: item.remark,
            })?;

            result_statuses.push(result.result_status);
            results.push(result);
        }

        let result_ids = self.result_repo.batch_create(&results).await?;

        lot.submit_results(command.inspector, now)?;
        self.lot_repo.update(&lot).await?;

        let results = result_ids
            .into_iter()
            .zip(result_statuses)
            .map(|(result_id, result_status)| AddInspectionResultOutput {
                result_id,
                result_status,
            })
            .collect();

        Ok(BatchAddInspectionResultsOutput { results })
    }
}

// =============================================================================
// 质量判定
// =============================================================================

/// 质量判定命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeInspectionDecisionCommand {
    pub inspection_lot_id: InspectionLotId,
    pub decision: InspectionDecision,
    pub reason: String,
    pub defect_code: Option<DefectCode>,
    pub create_notification: bool,
    pub notification_severity: Option<QualityNotificationSeverity>,
    pub decided_by: Operator,
    pub remark: Option<String>,
}

/// 质量判定输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeInspectionDecisionOutput {
    pub inspection_lot_id: InspectionLotId,
    pub decision: InspectionDecision,
    pub notification_id: Option<QualityNotificationId>,
}

/// 质量判定用例。
pub struct MakeInspectionDecisionUseCase<L, R, B, N, M, G>
where
    L: InspectionLotRepository,
    R: InspectionResultRepository,
    B: BatchQualityRepository,
    N: QualityNotificationRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub lot_repo: L,
    pub result_repo: R,
    pub batch_repo: B,
    pub notification_repo: N,
    pub master_repo: M,
    pub id_generator: G,
}

impl<L, R, B, N, M, G> MakeInspectionDecisionUseCase<L, R, B, N, M, G>
where
    L: InspectionLotRepository,
    R: InspectionResultRepository,
    B: BatchQualityRepository,
    N: QualityNotificationRepository,
    M: QualityMasterRepository,
    G: QualityIdGenerator,
{
    pub fn new(
        lot_repo: L,
        result_repo: R,
        batch_repo: B,
        notification_repo: N,
        master_repo: M,
        id_generator: G,
    ) -> Self {
        Self {
            lot_repo,
            result_repo,
            batch_repo,
            notification_repo,
            master_repo,
            id_generator,
        }
    }

    /// 执行质量判定。
    pub async fn execute(
        &self,
        command: MakeInspectionDecisionCommand,
    ) -> QualityResult<MakeInspectionDecisionOutput> {
        let now = OffsetDateTime::now_utc();

        if command.reason.trim().is_empty() {
            return Err(QualityError::QualityDecisionReasonRequired);
        }

        if let Some(defect_code) = &command.defect_code {
            let defect = self
                .master_repo
                .find_defect_code(defect_code)
                .await?
                .ok_or(QualityError::DefectCodeNotFound)?;

            if !defect.is_active {
                return Err(QualityError::DefectCodeInactive);
            }
        }

        let mut lot = self.lot_repo.lock_by_id(&command.inspection_lot_id).await?;

        if !lot.status.can_make_decision() {
            return Err(QualityError::InspectionLotStatusInvalid);
        }

        let has_any_result = self
            .result_repo
            .has_any_result(&command.inspection_lot_id)
            .await?;

        if !has_any_result {
            return Err(QualityError::InspectionResultRequired);
        }

        let has_failed_result = self
            .result_repo
            .has_failed_result(&command.inspection_lot_id)
            .await?;

        if command.decision == InspectionDecision::Accept && has_failed_result {
            return Err(QualityError::BusinessRuleViolation(
                "存在失败检验项，不能判定为 ACCEPT".to_string(),
            ));
        }

        let mut batch = self
            .batch_repo
            .lock_batch_for_update(&lot.batch_number)
            .await?;

        let old_status = batch.status;
        let new_status = command.decision.target_batch_status();

        lot.make_decision_with_reason(
            command.decision,
            command.reason.clone(),
            command.decided_by.clone(),
            now,
        )?;
        batch.status = new_status;

        self.lot_repo.update(&lot).await?;

        self.batch_repo
            .update_quality_status(&lot.batch_number, new_status)
            .await?;

        self.batch_repo
            .write_batch_history(&BatchQualityHistory {
                batch_number: lot.batch_number.clone(),
                old_status: Some(old_status),
                new_status,
                action: match command.decision {
                    InspectionDecision::Accept => BatchQualityAction::Accept,
                    InspectionDecision::Freeze => BatchQualityAction::Freeze,
                    InspectionDecision::Scrap => BatchQualityAction::Scrap,
                },
                reason: command.reason.clone(),
                reference_doc: Some(lot.id.to_string()),
                operator: command.decided_by.clone(),
                occurred_at: now,
                remark: command.remark.clone(),
            })
            .await?;

        let mut notification_id = None;

        if command.create_notification {
            let id = self.id_generator.next_quality_notification_id();

            let severity = command
                .notification_severity
                .unwrap_or(QualityNotificationSeverity::Medium);

            let notification = QualityNotification::create(CreateQualityNotification {
                id: id.clone(),
                source_type: "INSPECTION_LOT".to_string(),
                source_id: lot.id.to_string(),
                material_id: lot.material_id.clone(),
                batch_number: lot.batch_number.clone(),
                defect_code: command.defect_code,
                defect_qty: Decimal::ZERO,
                severity,
                description: command.reason.clone(),
                owner: None,
                created_by: command.decided_by,
                now,
                remark: command.remark,
            })?;

            self.notification_repo.create(&notification).await?;
            notification_id = Some(id);
        }

        Ok(MakeInspectionDecisionOutput {
            inspection_lot_id: command.inspection_lot_id,
            decision: command.decision,
            notification_id,
        })
    }
}

// =============================================================================
// 冻结批次
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeBatchCommand {
    pub batch_number: BatchNumber,
    pub reason: String,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeBatchOutput {
    pub batch_number: BatchNumber,
}

pub struct FreezeBatchUseCase<B>
where
    B: BatchQualityRepository,
{
    pub batch_repo: B,
}

impl<B> FreezeBatchUseCase<B>
where
    B: BatchQualityRepository,
{
    pub fn new(batch_repo: B) -> Self {
        Self { batch_repo }
    }

    pub async fn execute(&self, command: FreezeBatchCommand) -> QualityResult<FreezeBatchOutput> {
        if command.reason.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("reason"));
        }

        let now = OffsetDateTime::now_utc();

        let mut batch = self
            .batch_repo
            .lock_batch_for_update(&command.batch_number)
            .await?;

        let changed = batch.freeze()?;

        self.batch_repo
            .update_quality_status(&command.batch_number, changed.new_status)
            .await?;

        self.batch_repo
            .write_batch_history(&BatchQualityHistory {
                batch_number: command.batch_number.clone(),
                old_status: Some(changed.old_status),
                new_status: changed.new_status,
                action: changed.action,
                reason: command.reason,
                reference_doc: command.reference_doc,
                operator: command.operator,
                occurred_at: now,
                remark: command.remark,
            })
            .await?;

        Ok(FreezeBatchOutput {
            batch_number: command.batch_number,
        })
    }
}

// =============================================================================
// 解冻批次
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfreezeBatchCommand {
    pub batch_number: BatchNumber,

    /// 解冻后的目标状态。
    ///
    /// MVP 建议主要使用“合格”。
    /// 特殊情况下可以转回“待检”。
    pub target_status: BatchQualityStatus,

    pub reason: String,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfreezeBatchOutput {
    pub batch_number: BatchNumber,
    pub target_status: BatchQualityStatus,
}

pub struct UnfreezeBatchUseCase<B>
where
    B: BatchQualityRepository,
{
    pub batch_repo: B,
}

impl<B> UnfreezeBatchUseCase<B>
where
    B: BatchQualityRepository,
{
    pub fn new(batch_repo: B) -> Self {
        Self { batch_repo }
    }

    pub async fn execute(
        &self,
        command: UnfreezeBatchCommand,
    ) -> QualityResult<UnfreezeBatchOutput> {
        if command.reason.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("reason"));
        }

        let now = OffsetDateTime::now_utc();

        let mut batch = self
            .batch_repo
            .lock_batch_for_update(&command.batch_number)
            .await?;

        let changed = batch.unfreeze(command.target_status)?;

        self.batch_repo
            .update_quality_status(&command.batch_number, changed.new_status)
            .await?;

        self.batch_repo
            .write_batch_history(&BatchQualityHistory {
                batch_number: command.batch_number.clone(),
                old_status: Some(changed.old_status),
                new_status: changed.new_status,
                action: changed.action,
                reason: command.reason,
                reference_doc: command.reference_doc,
                operator: command.operator,
                occurred_at: now,
                remark: command.remark,
            })
            .await?;

        Ok(UnfreezeBatchOutput {
            batch_number: command.batch_number,
            target_status: command.target_status,
        })
    }
}

// =============================================================================
// 质量报废批次
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapBatchCommand {
    pub batch_number: BatchNumber,
    pub reason: String,
    pub defect_code: Option<DefectCode>,
    pub reference_doc: Option<String>,
    pub operator: Operator,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapBatchOutput {
    pub batch_number: BatchNumber,
}

pub struct ScrapBatchUseCase<B, M>
where
    B: BatchQualityRepository,
    M: QualityMasterRepository,
{
    pub batch_repo: B,
    pub master_repo: M,
}

impl<B, M> ScrapBatchUseCase<B, M>
where
    B: BatchQualityRepository,
    M: QualityMasterRepository,
{
    pub fn new(batch_repo: B, master_repo: M) -> Self {
        Self {
            batch_repo,
            master_repo,
        }
    }

    /// 执行质量报废。
    ///
    /// 注意：
    /// 这里只改变批次质量状态，不扣减库存。
    /// 实际扣库存应走库存模块的 999 报废出库。
    pub async fn execute(&self, command: ScrapBatchCommand) -> QualityResult<ScrapBatchOutput> {
        if command.reason.trim().is_empty() {
            return Err(QualityError::RequiredFieldEmpty("reason"));
        }

        if let Some(defect_code) = &command.defect_code {
            let defect = self
                .master_repo
                .find_defect_code(defect_code)
                .await?
                .ok_or(QualityError::DefectCodeNotFound)?;

            if !defect.is_active {
                return Err(QualityError::DefectCodeInactive);
            }
        }

        let now = OffsetDateTime::now_utc();

        let mut batch = self
            .batch_repo
            .lock_batch_for_update(&command.batch_number)
            .await?;

        let changed = batch.scrap()?;

        self.batch_repo
            .update_quality_status(&command.batch_number, changed.new_status)
            .await?;

        self.batch_repo
            .write_batch_history(&BatchQualityHistory {
                batch_number: command.batch_number.clone(),
                old_status: Some(changed.old_status),
                new_status: changed.new_status,
                action: changed.action,
                reason: command.reason,
                reference_doc: command.reference_doc,
                operator: command.operator,
                occurred_at: now,
                remark: command.remark,
            })
            .await?;

        Ok(ScrapBatchOutput {
            batch_number: command.batch_number,
        })
    }
}
