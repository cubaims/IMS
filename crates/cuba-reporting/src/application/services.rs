use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct ReportingService {
    state: AppState,
}

impl ReportingService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("reporting module ready")
    }
}
