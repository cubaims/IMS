use crate::domain::{
    CreateMrpRun, MaterialId, MrpError, MrpResult, MrpRun, MrpRunId, MrpRunStatus, MrpSuggestion,
    MrpSuggestionId, MrpSuggestionStatus, MrpSuggestionType, Operator, ProductVariantId,
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
    pub only_shortage: Option<bool>,
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
    async fn find_by_id(&self, suggestion_id: &MrpSuggestionId)
    -> MrpResult<Option<MrpSuggestion>>;

    /// 锁定 MRP 建议。
    async fn lock_by_id(&self, suggestion_id: &MrpSuggestionId) -> MrpResult<MrpSuggestion>;

    /// 更新 MRP 建议。
    async fn update(&self, suggestion: &MrpSuggestion) -> MrpResult<()>;

    /// 分页查询 MRP 建议。
    async fn list(&self, query: MrpSuggestionQuery) -> MrpResult<Page<MrpSuggestion>>;
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
    async fn run_mrp_function(&self, run: &MrpRun) -> MrpResult<MrpRunId>;
}

/// MRP 主数据校验端口。
#[async_trait]
pub trait MrpMasterRepository: Send + Sync {
    /// 物料是否存在且启用。
    async fn material_exists_and_active(&self, material_id: &MaterialId) -> MrpResult<bool>;

    /// 产品变体是否存在。
    async fn product_variant_exists(
        &self,
        product_variant_id: &ProductVariantId,
    ) -> MrpResult<bool>;

    /// 按成品物料查找可用于 MRP 的启用产品变体。
    async fn find_active_variant_by_material(
        &self,
        material_id: &MaterialId,
    ) -> MrpResult<Option<ProductVariantId>>;
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
    pub product_variant_id: Option<ProductVariantId>,
}

/// 查询单个 MRP 运行记录用例。
pub struct GetMrpRunUseCase<R>
where
    R: MrpRunRepository,
{
    pub run_repo: R,
}

impl<R> GetMrpRunUseCase<R>
where
    R: MrpRunRepository,
{
    pub fn new(run_repo: R) -> Self {
        Self { run_repo }
    }

    pub async fn execute(&self, run_id: MrpRunId) -> MrpResult<MrpRun> {
        self.run_repo
            .find_by_id(&run_id)
            .await?
            .ok_or(MrpError::MrpRunNotFound)
    }
}

/// 分页查询 MRP 运行记录用例。
pub struct ListMrpRunsUseCase<R>
where
    R: MrpRunRepository,
{
    pub run_repo: R,
}

impl<R> ListMrpRunsUseCase<R>
where
    R: MrpRunRepository,
{
    pub fn new(run_repo: R) -> Self {
        Self { run_repo }
    }

    pub async fn execute(&self, query: MrpRunQuery) -> MrpResult<Page<MrpRunSummary>> {
        self.run_repo.list(query).await
    }
}

/// 查询单个 MRP 建议用例。
pub struct GetMrpSuggestionUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub suggestion_repo: S,
}

impl<S> GetMrpSuggestionUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub fn new(suggestion_repo: S) -> Self {
        Self { suggestion_repo }
    }

    pub async fn execute(&self, suggestion_id: MrpSuggestionId) -> MrpResult<MrpSuggestion> {
        self.suggestion_repo
            .find_by_id(&suggestion_id)
            .await?
            .ok_or(MrpError::MrpSuggestionNotFound)
    }
}

/// 分页查询 MRP 建议用例。
pub struct ListMrpSuggestionsUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub suggestion_repo: S,
}

impl<S> ListMrpSuggestionsUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub fn new(suggestion_repo: S) -> Self {
        Self { suggestion_repo }
    }

    pub async fn execute(&self, query: MrpSuggestionQuery) -> MrpResult<Page<MrpSuggestion>> {
        self.suggestion_repo.list(query).await
    }
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
    pub fn new(run_repo: R, planner_gateway: P, master_repo: M, id_generator: G) -> Self {
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

        let product_variant_id = if let Some(product_variant_id) = &command.product_variant_id {
            let exists = self
                .master_repo
                .product_variant_exists(product_variant_id)
                .await?;

            if !exists {
                return Err(MrpError::ProductVariantNotFound);
            }

            Some(product_variant_id.clone())
        } else if let Some(material_id) = &command.material_id {
            self.master_repo
                .find_active_variant_by_material(material_id)
                .await?
        } else {
            None
        };

        let Some(product_variant_id) = product_variant_id else {
            return Err(MrpError::ProductVariantRequired);
        };

        // 这里的 id 只是为了构造领域对象。
        // 真正落库的 run_id 由 wms.fn_run_mrp() 返回。
        let temp_run_id = self.id_generator.next_mrp_run_id();

        let run = MrpRun::create(CreateMrpRun {
            id: temp_run_id,
            material_id: command.material_id,
            product_variant_id: Some(product_variant_id.clone()),
            demand_qty: command.demand_qty,
            demand_date: command.demand_date,
            created_by: command.created_by,
            now,
            remark: command.remark,
        })?;

        let db_run_id = self.planner_gateway.run_mrp_function(&run).await?;

        let status = match self.run_repo.find_by_id(&db_run_id).await? {
            Some(saved_run) => saved_run.status,
            None => MrpRunStatus::Completed,
        };

        Ok(RunMrpOutput {
            run_id: db_run_id,
            status,
            product_variant_id: Some(product_variant_id),
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
    pub remark: Option<String>,
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
        suggestion.remark = command.remark;

        self.suggestion_repo.update(&suggestion).await?;

        Ok(ConfirmMrpSuggestionOutput {
            suggestion_id: command.suggestion_id,
            status: suggestion.status,
        })
    }
}

// =============================================================================
// Use Case：取消 MRP 建议
// =============================================================================

/// 取消 MRP 建议命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelMrpSuggestionCommand {
    pub suggestion_id: MrpSuggestionId,
    pub cancelled_by: Operator,
    pub reason: String,
}

/// 取消 MRP 建议输出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelMrpSuggestionOutput {
    pub suggestion_id: MrpSuggestionId,
    pub status: MrpSuggestionStatus,
}

/// 取消 MRP 建议用例。
pub struct CancelMrpSuggestionUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub suggestion_repo: S,
}

impl<S> CancelMrpSuggestionUseCase<S>
where
    S: MrpSuggestionRepository,
{
    pub fn new(suggestion_repo: S) -> Self {
        Self { suggestion_repo }
    }

    /// 执行取消。
    pub async fn execute(
        &self,
        command: CancelMrpSuggestionCommand,
    ) -> MrpResult<CancelMrpSuggestionOutput> {
        let now = OffsetDateTime::now_utc();

        let mut suggestion = self
            .suggestion_repo
            .lock_by_id(&command.suggestion_id)
            .await?;

        suggestion.cancel(command.cancelled_by, now, command.reason)?;

        self.suggestion_repo.update(&suggestion).await?;

        Ok(CancelMrpSuggestionOutput {
            suggestion_id: command.suggestion_id,
            status: suggestion.status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct InMemoryRuns {
        runs: Arc<Mutex<Vec<MrpRun>>>,
    }

    #[derive(Clone, Default)]
    struct InMemoryMaster {
        materials: Arc<Mutex<Vec<MaterialId>>>,
        variants: Arc<Mutex<Vec<(ProductVariantId, MaterialId)>>>,
    }

    #[derive(Clone, Default)]
    struct InMemoryPlanner {
        runs: Arc<Mutex<Vec<MrpRun>>>,
    }

    #[derive(Clone, Default)]
    struct StaticIdGenerator;

    #[async_trait]
    impl MrpRunRepository for InMemoryRuns {
        async fn create(&self, run: &MrpRun) -> MrpResult<MrpRunId> {
            self.runs.lock().expect("runs lock").push(run.clone());
            Ok(run.id.clone())
        }

        async fn find_by_id(&self, run_id: &MrpRunId) -> MrpResult<Option<MrpRun>> {
            Ok(self
                .runs
                .lock()
                .expect("runs lock")
                .iter()
                .find(|run| run.id == *run_id)
                .cloned())
        }

        async fn lock_by_id(&self, run_id: &MrpRunId) -> MrpResult<MrpRun> {
            self.find_by_id(run_id)
                .await?
                .ok_or(MrpError::MrpRunNotFound)
        }

        async fn update(&self, run: &MrpRun) -> MrpResult<()> {
            let mut runs = self.runs.lock().expect("runs lock");
            let Some(saved) = runs.iter_mut().find(|saved| saved.id == run.id) else {
                return Err(MrpError::MrpRunNotFound);
            };
            *saved = run.clone();
            Ok(())
        }

        async fn list(&self, query: MrpRunQuery) -> MrpResult<Page<MrpRunSummary>> {
            let runs = self.runs.lock().expect("runs lock");
            let mut items = Vec::new();

            for run in runs.iter() {
                if let Some(status) = query.status {
                    if run.status != status {
                        continue;
                    }
                }

                items.push(MrpRunSummary {
                    id: run.id.clone(),
                    material_id: run.material_id.clone(),
                    product_variant_id: run.product_variant_id.clone(),
                    demand_qty: run.demand_qty,
                    demand_date: run.demand_date,
                    status: run.status,
                    created_at: run.created_at,
                });
            }

            Ok(Page::new(
                items.clone(),
                items.len() as u64,
                query.page.page,
                query.page.page_size,
            ))
        }
    }

    impl MrpIdGenerator for StaticIdGenerator {
        fn next_mrp_run_id(&self) -> MrpRunId {
            MrpRunId::new("MRP-TEMP")
        }
    }

    #[async_trait]
    impl MrpPlannerGateway for InMemoryPlanner {
        async fn run_mrp_function(&self, run: &MrpRun) -> MrpResult<MrpRunId> {
            self.runs
                .lock()
                .expect("planner runs lock")
                .push(run.clone());
            Ok(MrpRunId::new("MRP-DB"))
        }
    }

    #[async_trait]
    impl MrpMasterRepository for InMemoryMaster {
        async fn material_exists_and_active(&self, material_id: &MaterialId) -> MrpResult<bool> {
            Ok(self
                .materials
                .lock()
                .expect("materials lock")
                .iter()
                .any(|saved| saved == material_id))
        }

        async fn product_variant_exists(
            &self,
            product_variant_id: &ProductVariantId,
        ) -> MrpResult<bool> {
            Ok(self
                .variants
                .lock()
                .expect("variants lock")
                .iter()
                .any(|(variant_id, _)| variant_id == product_variant_id))
        }

        async fn find_active_variant_by_material(
            &self,
            material_id: &MaterialId,
        ) -> MrpResult<Option<ProductVariantId>> {
            Ok(self
                .variants
                .lock()
                .expect("variants lock")
                .iter()
                .find(|(_, saved_material_id)| saved_material_id == material_id)
                .map(|(variant_id, _)| variant_id.clone()))
        }
    }

    #[derive(Clone, Default)]
    struct InMemorySuggestions {
        suggestions: Arc<Mutex<Vec<MrpSuggestion>>>,
    }

    #[async_trait]
    impl MrpSuggestionRepository for InMemorySuggestions {
        async fn find_by_id(
            &self,
            suggestion_id: &MrpSuggestionId,
        ) -> MrpResult<Option<MrpSuggestion>> {
            Ok(self
                .suggestions
                .lock()
                .expect("suggestions lock")
                .iter()
                .find(|suggestion| suggestion.id == *suggestion_id)
                .cloned())
        }

        async fn lock_by_id(&self, suggestion_id: &MrpSuggestionId) -> MrpResult<MrpSuggestion> {
            self.find_by_id(suggestion_id)
                .await?
                .ok_or(MrpError::MrpSuggestionNotFound)
        }

        async fn update(&self, suggestion: &MrpSuggestion) -> MrpResult<()> {
            let mut suggestions = self.suggestions.lock().expect("suggestions lock");
            let Some(saved) = suggestions
                .iter_mut()
                .find(|saved| saved.id == suggestion.id)
            else {
                return Err(MrpError::MrpSuggestionNotFound);
            };
            *saved = suggestion.clone();
            Ok(())
        }

        async fn list(&self, query: MrpSuggestionQuery) -> MrpResult<Page<MrpSuggestion>> {
            let suggestions = self.suggestions.lock().expect("suggestions lock");
            let mut items = Vec::new();

            for suggestion in suggestions.iter() {
                if let Some(status) = query.status {
                    if suggestion.status != status {
                        continue;
                    }
                }
                items.push(suggestion.clone());
            }

            Ok(Page::new(
                items.clone(),
                items.len() as u64,
                query.page.page,
                query.page.page_size,
            ))
        }
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH
    }

    fn run(id: &str, status: MrpRunStatus) -> MrpRun {
        MrpRun {
            id: MrpRunId::new(id),
            material_id: Some(MaterialId::new("FIN001")),
            product_variant_id: Some(ProductVariantId::new("FIN-A001")),
            demand_qty: Decimal::ONE,
            demand_date: now(),
            status,
            created_by: Operator::new("planner"),
            created_at: now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            remark: None,
        }
    }

    fn suggestion(id: &str, status: MrpSuggestionStatus) -> MrpSuggestion {
        MrpSuggestion {
            id: MrpSuggestionId::new(id),
            run_id: MrpRunId::new("MRP-1"),
            suggestion_type: MrpSuggestionType::Purchase,
            material_id: MaterialId::new("RM001"),
            bom_level: 1,
            gross_requirement_qty: Decimal::ONE,
            required_qty: Decimal::ONE,
            available_qty: Decimal::ZERO,
            safety_stock_qty: Decimal::ZERO,
            shortage_qty: Decimal::ONE,
            net_requirement_qty: Decimal::ONE,
            suggested_qty: Decimal::ONE,
            recommended_bin: None,
            recommended_batch: None,
            lead_time_days: None,
            priority: None,
            required_date: now(),
            suggested_date: now(),
            supplier_id: None,
            work_center_id: None,
            status,
            created_at: now(),
            confirmed_by: None,
            confirmed_at: None,
            cancelled_by: None,
            cancelled_at: None,
            remark: None,
        }
    }

    #[tokio::test]
    async fn list_and_get_run_use_cases_delegate_to_repository() {
        let repository = InMemoryRuns::default();
        repository.runs.lock().expect("runs lock").extend([
            run("MRP-1", MrpRunStatus::Completed),
            run("MRP-2", MrpRunStatus::Running),
        ]);

        let get = GetMrpRunUseCase::new(repository.clone());
        let found = get
            .execute(MrpRunId::new("MRP-1"))
            .await
            .expect("run exists");
        assert_eq!(found.status, MrpRunStatus::Completed);

        let list = ListMrpRunsUseCase::new(repository);
        let page = list
            .execute(MrpRunQuery {
                page: PageQuery {
                    page: 1,
                    page_size: 20,
                },
                status: Some(MrpRunStatus::Completed),
                material_id: None,
                product_variant_id: None,
                date_from: None,
                date_to: None,
            })
            .await
            .expect("list runs");

        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id.as_str(), "MRP-1");
    }

    #[tokio::test]
    async fn suggestion_use_cases_confirm_and_cancel_by_status() {
        let repository = InMemorySuggestions::default();
        repository
            .suggestions
            .lock()
            .expect("suggestions lock")
            .push(suggestion("1", MrpSuggestionStatus::Open));

        let confirm = ConfirmMrpSuggestionUseCase::new(repository.clone());
        let output = confirm
            .execute(ConfirmMrpSuggestionCommand {
                suggestion_id: MrpSuggestionId::new("1"),
                confirmed_by: Operator::new("planner"),
                remark: Some("ok".to_string()),
            })
            .await
            .expect("confirm");
        assert_eq!(output.status, MrpSuggestionStatus::Confirmed);

        let cancel = CancelMrpSuggestionUseCase::new(repository.clone());
        let output = cancel
            .execute(CancelMrpSuggestionCommand {
                suggestion_id: MrpSuggestionId::new("1"),
                cancelled_by: Operator::new("planner"),
                reason: "demand cancelled".to_string(),
            })
            .await
            .expect("cancel");
        assert_eq!(output.status, MrpSuggestionStatus::Cancelled);

        let get = GetMrpSuggestionUseCase::new(repository);
        let saved = get
            .execute(MrpSuggestionId::new("1"))
            .await
            .expect("suggestion exists");
        assert_eq!(saved.status, MrpSuggestionStatus::Cancelled);
    }

    #[tokio::test]
    async fn run_mrp_resolves_variant_from_finished_material() {
        let run_repo = InMemoryRuns::default();
        let planner = InMemoryPlanner::default();
        let master = InMemoryMaster::default();
        master
            .materials
            .lock()
            .expect("materials lock")
            .push(MaterialId::new("FIN001"));
        master
            .variants
            .lock()
            .expect("variants lock")
            .push((ProductVariantId::new("FIN-A001"), MaterialId::new("FIN001")));

        let use_case = RunMrpUseCase::new(run_repo, planner.clone(), master, StaticIdGenerator);

        let output = use_case
            .execute(RunMrpCommand {
                material_id: Some(MaterialId::new("FIN001")),
                product_variant_id: None,
                demand_qty: Decimal::ONE,
                demand_date: now(),
                created_by: Operator::new("planner"),
                remark: None,
            })
            .await
            .expect("run mrp by material");

        assert_eq!(output.run_id.as_str(), "MRP-DB");
        assert_eq!(
            output
                .product_variant_id
                .expect("resolved variant")
                .as_str(),
            "FIN-A001"
        );
        assert_eq!(
            planner.runs.lock().expect("planner runs lock")[0]
                .product_variant_id
                .as_ref()
                .expect("run variant")
                .as_str(),
            "FIN-A001"
        );
    }
}
