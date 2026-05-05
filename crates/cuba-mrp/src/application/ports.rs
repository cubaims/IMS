use crate::domain::{
    CreateMrpRun, MaterialId, MrpError, MrpResult, MrpRun, MrpRunId,
    MrpRunStatus, MrpSuggestion, MrpSuggestionId, MrpSuggestionStatus,
    MrpSuggestionType, Operator, ProductVariantId,
};
use async_trait::async_trait;
use cuba_shared::{Page, PageQuery};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// =============================================================================
// ID 生成器
// =============================================================================

/// MRP 模块 ID 生成器。
pub trait MrpIdGenerator: Send + Sync {
    fn next_mrp_run_id(&self) -> MrpRunId;
}

// =============================================================================
// Repository Ports
// =============================================================================

/// MRP 运行查询条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpRunQuery {
    pub page: PageQuery,
    pub status: Option<MrpRunStatus>,
    pub material_id: Option<MaterialId>,
    pub product_variant_id: Option<ProductVariantId>,
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
}

/// MRP 运行摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpRunSummary {
    pub id: MrpRunId,
    pub material_id: Option<MaterialId>,
    pub product_variant_id: Option<ProductVariantId>,
    pub demand_qty: Decimal,
    pub demand_date: OffsetDateTime,
    pub status: MrpRunStatus,
    pub created_at: OffsetDateTime,
}

/// MRP 建议查询条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpSuggestionQuery {
    pub page: PageQuery,
    pub run_id: Option<MrpRunId>,
    pub suggestion_type: Option<MrpSuggestionType>,
    pub status: Option<MrpSuggestionStatus>,
    pub material_id: Option<MaterialId>,
    pub required_date_from: Option<OffsetDateTime>,
    pub required_date_to: Option<OffsetDateTime>,
}

/// MRP 运行仓储。
#[async_trait]
pub trait MrpRunRepository: Send + Sync {
    /// 创建 MRP 运行记录。
    async fn create(&self, run: &MrpRun) -> MrpResult<MrpRunId>;

    /// 查询 MRP 运行记录。
    async fn find_by_id(&self, run_id: &MrpRunId) -> MrpResult<Option<MrpRun>>;

    /// 锁定 MRP 运行记录。
    async fn lock_by_id(&self, run_id: &MrpRunId) -> MrpResult<MrpRun>;

    /// 更新 MRP 运行记录。
    async fn update(&self, run: &MrpRun) -> MrpResult<()>;

    /// 分页查询 MRP 运行记录。
    async fn list(&self, query: MrpRunQuery) -> MrpResult<Page<MrpRunSummary>>;
}

/// MRP 建议仓储。
#[async_trait]
pub trait MrpSuggestionRepository: Send + Sync {
    /// 查询 MRP 建议。
    async fn find_by_id(
        &self,
        suggestion_id: &MrpSuggestionId,
    ) -> MrpResult<Option<MrpSuggestion>>;

    /// 锁定 MRP 建议。
    async fn lock_by_id(
        &self,
        suggestion_id: &MrpSuggestionId,
    ) -> MrpResult<MrpSuggestion>;

    /// 更新 MRP 建议。
    async fn update(&self, suggestion: &MrpSuggestion) -> MrpResult<()>;

    /// 分页查询 MRP 建议。
    async fn list(
        &self,
        query: MrpSuggestionQuery,
    ) -> MrpResult<Page<MrpSuggestion>>;
}

/// MRP 数据库函数端口。
///
/// 核心 MRP 计算不在 Rust 里重写，
/// 而是调用 PostgreSQL 函数 fn_run_mrp()。
#[async_trait]
pub trait MrpPlannerGateway: Send + Sync {
    /// 运行数据库 MRP 函数。
    ///
    /// 返回 run_id 或数据库生成的运行结果标识。
    async fn run_mrp_function(
        &self,
        run: &MrpRun,
    ) -> MrpResult<()>;
}

/// MRP 主数据校验端口。
#[async_trait]
pub trait MrpMasterRepository: Send + Sync {
    /// 物料是否存在且启用。
    async fn material_exists_and_active(
        &self,
        material_id: &MaterialId,
    ) -> MrpResult<bool>;

    /// 产品变体是否存在。
    async fn product_variant_exists(
        &self,
        product_variant_id: &ProductVariantId,
    ) -> MrpResult<bool>;
}

// =============================================================================
// Use Case：运行 MRP
// =============================================================================

/// 运行 MRP 命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMrpCommand {
    pub material_id: Option<MaterialId>,
    pub product_variant_id: Option<ProductVariantId>,
    pub demand_qty: Decimal,
    pub demand_date: OffsetDateTime,
    pub created_by: Operator,
    pub remark: Option<String>,
}

/// 运行 MRP 输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMrpOutput {
    pub run_id: MrpRunId,
    pub status: MrpRunStatus,
}

/// 运行 MRP 用例。
pub struct RunMrpUseCase<R, P, M, G>
where
    R: MrpRunRepository,
    P: MrpPlannerGateway,
    M: MrpMasterRepository,
    G: MrpIdGenerator,
{
    pub run_repo: R,
    pub planner_gateway: P,
    pub master_repo: M,
    pub id_generator: G,
}

impl<R, P, M, G> RunMrpUseCase<R, P, M, G>
where
    R: MrpRunRepository,
    P: MrpPlannerGateway,
    M: MrpMasterRepository,
    G: MrpIdGenerator,
{
    pub fn new(
        run_repo: R,
        planner_gateway: P,
        master_repo: M,
        id_generator: G,
    ) -> Self {
        Self {
            run_repo,
            planner_gateway,
            master_repo,
            id_generator,
        }
    }

    /// 执行 MRP。
    ///
    /// 事务建议：
    /// BEGIN
    ///   校验物料 / 产品变体
    ///   创建 MRP run
    ///   标记 running
    ///   调用 fn_run_mrp()
    ///   标记 completed / failed
    /// COMMIT
    pub async fn execute(&self, command: RunMrpCommand) -> MrpResult<RunMrpOutput> {
        let now = OffsetDateTime::now_utc();

        if command.demand_qty <= Decimal::ZERO {
            return Err(MrpError::DemandQtyMustBePositive);
        }

        if let Some(material_id) = &command.material_id {
            let exists = self
                .master_repo
                .material_exists_and_active(material_id)
                .await?;

            if !exists {
                return Err(MrpError::MaterialNotFoundOrInactive);
            }
        }

        if let Some(product_variant_id) = &command.product_variant_id {
            let exists = self
                .master_repo
                .product_variant_exists(product_variant_id)
                .await?;

            if !exists {
                return Err(MrpError::ProductVariantNotFound);
            }
        }

        let run_id = self.id_generator.next_mrp_run_id();

        let mut run = MrpRun::create(CreateMrpRun {
            id: run_id.clone(),
            material_id: command.material_id,
            product_variant_id: command.product_variant_id,
            demand_qty: command.demand_qty,
            demand_date: command.demand_date,
            created_by: command.created_by,
            now,
            remark: command.remark,
        })?;

        self.run_repo.create(&run).await?;

        run.mark_running(now);
        self.run_repo.update(&run).await?;

        let planner_result = self.planner_gateway.run_mrp_function(&run).await;

        match planner_result {
            Ok(_) => {
                run.mark_completed(OffsetDateTime::now_utc());
                self.run_repo.update(&run).await?;
            }
            Err(error) => {
                run.mark_failed(
                    OffsetDateTime::now_utc(),
                    error.to_string(),
                );

                self.run_repo.update(&run).await?;

                return Err(MrpError::MrpRunFailed);
            }
        }

        Ok(RunMrpOutput {
            run_id,
            status: run.status,
        })
    }
}

// =============================================================================
// Use Case：确认 MRP 建议
// =============================================================================

/// 确认 MRP 建议命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmMrpSuggestionCommand {
    pub suggestion_id: MrpSuggestionId,
    pub confirmed_by: Operator,
}

/// 确认 MRP 建议输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmMrpSuggestionOutput {
    pub suggestion_id: MrpSuggestionId,
    pub status: MrpSuggestionStatus,
}

/// 确认 MRP 建议用例。
pub struct ConfirmMrpSuggestionUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub suggestion_repo: S,
}

impl<S> ConfirmMrpSuggestionUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub fn new(suggestion_repo: S) -> Self {
        Self { suggestion_repo }
    }

    /// 执行确认。
    pub async fn execute(
        &self,
        command: ConfirmMrpSuggestionCommand,
    ) -> MrpResult<ConfirmMrpSuggestionOutput> {
        let now = OffsetDateTime::now_utc();

        let mut suggestion = self
            .suggestion_repo
            .lock_by_id(&command.suggestion_id)
            .await?;

        suggestion.confirm(command.confirmed_by, now)?;

        self.suggestion_repo.update(&suggestion).await?;

        Ok(ConfirmMrpSuggestionOutput {
            suggestion_id: command.suggestion_id,
            status: suggestion.status,
        })
    }
}