use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};
use crate::application::PurchaseRepository;

#[derive(Clone)]
pub struct PostgresPurchaseRepository {
    state: AppState,
}

impl PostgresPurchaseRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl PurchaseRepository for PostgresPurchaseRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
