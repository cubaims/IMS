use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct AuthService {
    state: AppState,
}

impl AuthService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn health(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("auth module ready")
    }
}
