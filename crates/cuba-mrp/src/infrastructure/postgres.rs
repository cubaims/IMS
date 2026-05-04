use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};
use crate::application::MrpRepository;

#[derive(Clone)]
pub struct PostgresMrpRepository {
    state: AppState,
}

impl PostgresMrpRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MrpRepository for PostgresMrpRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
