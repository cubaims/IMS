use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};
use crate::application::SalesRepository;

#[derive(Clone)]
pub struct PostgresSalesRepository {
    state: AppState,
}

impl PostgresSalesRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SalesRepository for PostgresSalesRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
