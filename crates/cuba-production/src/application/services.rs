use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct ProductionService {
    state: AppState,
}

impl ProductionService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("production module ready")
    }
}
