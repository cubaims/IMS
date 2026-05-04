use crate::application::ReportingRepository;
use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct PostgresReportingRepository {
    state: AppState,
}

impl PostgresReportingRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ReportingRepository for PostgresReportingRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
