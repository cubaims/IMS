use crate::application::QualityRepository;
use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct PostgresQualityRepository {
    state: AppState,
}

impl PostgresQualityRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl QualityRepository for PostgresQualityRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
