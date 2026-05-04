use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};
use crate::application::ProductionRepository;

#[derive(Clone)]
pub struct PostgresProductionRepository {
    state: AppState,
}

impl PostgresProductionRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ProductionRepository for PostgresProductionRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
