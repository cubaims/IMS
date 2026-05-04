use crate::application::AuthRepository;
use async_trait::async_trait;
use cuba_shared::{AppResult, AppState};

#[derive(Clone)]
pub struct PostgresAuthRepository {
    state: AppState,
}

impl PostgresAuthRepository {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn ping(&self) -> AppResult<&'static str> {
        let _ = &self.state;
        Ok("ok")
    }
}
