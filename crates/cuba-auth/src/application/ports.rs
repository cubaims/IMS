use async_trait::async_trait;
use cuba_shared::AppResult;

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn ping(&self) -> AppResult<&'static str>;
}
