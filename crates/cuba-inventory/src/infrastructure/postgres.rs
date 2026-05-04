use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};
use crate::application::InventoryRepository;

#[derive(Clone)]
pub struct PostgresInventoryRepository {
    state: AppState,
}

impl PostgresInventoryRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl InventoryRepository for PostgresInventoryRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
