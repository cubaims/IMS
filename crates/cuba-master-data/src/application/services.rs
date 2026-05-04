use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct MasterDataService {
    state: AppState,
}

impl MasterDataService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("master_data module ready")
    }
}
