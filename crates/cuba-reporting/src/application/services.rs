use crate::application::{
    MaterializedViewRepository, RefreshMaterializedViewsCommand, ReportExportRepository,
    ReportingRepository,
};
use crate::domain::{
    ExportedReport, ReportExportRequest, ReportFilters, ReportPage, ReportQuery,
    ReportRefreshResult,
};
use cuba_shared::{AppError, AppResult, AppState};

#[derive(Clone)]
pub struct ReportingService {
    state: AppState,
}

impl ReportingService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("reporting module ready")
    }
}

pub struct RefreshMaterializedViewsUseCase<R>
where
    R: MaterializedViewRepository,
{
    repository: R,
}

impl<R> RefreshMaterializedViewsUseCase<R>
where
    R: MaterializedViewRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: RefreshMaterializedViewsCommand,
    ) -> AppResult<ReportRefreshResult> {
        self.repository.refresh_all(command).await
    }
}

pub struct GetReportUseCase<R>
where
    R: ReportingRepository,
{
    repository: R,
}

impl<R> GetReportUseCase<R>
where
    R: ReportingRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, query: ReportQuery) -> AppResult<ReportPage> {
        validate_report_query(&query)?;
        self.repository.query_report(query).await
    }
}

pub struct ExportReportUseCase<R>
where
    R: ReportExportRepository,
{
    repository: R,
}

impl<R> ExportReportUseCase<R>
where
    R: ReportExportRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: ReportExportRequest) -> AppResult<ExportedReport> {
        self.repository.export_report(request).await
    }
}

fn validate_report_query(query: &ReportQuery) -> AppResult<()> {
    if let ReportFilters::MrpShortage(filter) = &query.filters {
        if let (Some(date_from), Some(date_to)) = (filter.date_from, filter.date_to) {
            if date_from >= date_to {
                return Err(AppError::business(
                    "REPORT_QUERY_INVALID",
                    "date_from 必须早于 date_to",
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ExportedReport, MrpShortageReportFilter, ReportExportFormat, ReportFilters, ReportType,
    };
    use async_trait::async_trait;
    use cuba_shared::{Page, PageQuery};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use time::OffsetDateTime;

    #[derive(Clone, Default)]
    struct InMemoryReports {
        queries: Arc<Mutex<Vec<ReportQuery>>>,
    }

    #[async_trait]
    impl ReportingRepository for InMemoryReports {
        async fn ping(&self) -> AppResult<&'static str> {
            Ok("ok")
        }

        async fn query_report(&self, query: ReportQuery) -> AppResult<ReportPage> {
            self.queries.lock().expect("queries lock").push(query);
            Ok(Page::new(vec![json!({"material_id": "RM001"})], 1, 1, 20))
        }
    }

    #[async_trait]
    impl ReportExportRepository for InMemoryReports {
        async fn export_report(&self, request: ReportExportRequest) -> AppResult<ExportedReport> {
            Ok(ExportedReport {
                filename: format!("{}.csv", request.report_type.view_name()),
                content_type: "text/csv; charset=utf-8".to_string(),
                body: "material_id\nRM001\n".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn get_report_use_case_validates_mrp_shortage_date_range() {
        let use_case = GetReportUseCase::new(InMemoryReports::default());
        let err = use_case
            .execute(ReportQuery {
                report_type: ReportType::MrpShortage,
                filters: ReportFilters::MrpShortage(MrpShortageReportFilter {
                    date_from: Some(OffsetDateTime::UNIX_EPOCH),
                    date_to: Some(OffsetDateTime::UNIX_EPOCH),
                    ..MrpShortageReportFilter::default()
                }),
                page: PageQuery {
                    page: 1,
                    page_size: 20,
                },
            })
            .await
            .expect_err("invalid range should fail");

        assert_eq!(err.error_code(), "REPORT_QUERY_INVALID");
    }

    #[tokio::test]
    async fn get_report_use_case_returns_page_from_repository() {
        let repository = InMemoryReports::default();
        let use_case = GetReportUseCase::new(repository.clone());

        let page = use_case
            .execute(ReportQuery {
                report_type: ReportType::CurrentStock,
                filters: ReportFilters::CurrentStock(Default::default()),
                page: PageQuery {
                    page: 1,
                    page_size: 20,
                },
            })
            .await
            .expect("query succeeds");

        assert_eq!(page.total, 1);
        assert_eq!(
            repository.queries.lock().expect("queries lock")[0].report_type,
            ReportType::CurrentStock
        );
    }

    #[tokio::test]
    async fn export_report_use_case_returns_exported_file() {
        let use_case = ExportReportUseCase::new(InMemoryReports::default());
        let exported = use_case
            .execute(ReportExportRequest {
                report_type: ReportType::CurrentStock,
                filters: ReportFilters::CurrentStock(Default::default()),
                format: ReportExportFormat::Csv,
                include_headers: true,
            })
            .await
            .expect("export succeeds");

        assert_eq!(exported.content_type, "text/csv; charset=utf-8");
        assert!(exported.body.contains("RM001"));
    }
}
