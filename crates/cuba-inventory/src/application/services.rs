use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct InventoryService {
    state: AppState,
}

impl InventoryService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("inventory module ready")
    }
}
