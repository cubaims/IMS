use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct MrpService {
    state: AppState,
}

impl MrpService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("mrp module ready")
    }
}
