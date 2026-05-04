use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct PurchaseService {
    state: AppState,
}

impl PurchaseService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("purchase module ready")
    }
}
