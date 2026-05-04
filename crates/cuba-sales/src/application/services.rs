use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct SalesService {
    state: AppState,
}

impl SalesService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("sales module ready")
    }
}
