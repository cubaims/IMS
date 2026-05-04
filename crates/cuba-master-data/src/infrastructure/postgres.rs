use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};
use crate::application::MasterDataRepository;

#[derive(Clone)]
pub struct PostgresMasterDataRepository {
    state: AppState,
}

impl PostgresMasterDataRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MasterDataRepository for PostgresMasterDataRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
