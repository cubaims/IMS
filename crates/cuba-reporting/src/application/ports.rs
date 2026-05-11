use crate::application::RefreshMaterializedViewsCommand;
use crate::domain::{
    ExportedReport, ReportExportRequest, ReportPage, ReportQuery, ReportRefreshResult,
};
use async_trait::async_trait;
use cuba_shared::AppResult;

#[async_trait]
pub trait ReportingRepository: Send + Sync {
    async fn ping(&self) -> AppResult<&'static str>;

    async fn query_report(&self, query: ReportQuery) -> AppResult<ReportPage>;
}

#[async_trait]
pub trait MaterializedViewRepository: Send + Sync {
    async fn refresh_all(
        &self,
        command: RefreshMaterializedViewsCommand,
    ) -> AppResult<ReportRefreshResult>;
}

#[async_trait]
pub trait ReportExportRepository: Send + Sync {
    async fn export_report(&self, request: ReportExportRequest) -> AppResult<ExportedReport>;
}
