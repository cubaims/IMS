use async_trait::async_trait;
use cuba_shared::AppResult;

#[async_trait]
pub trait ReportingRepository: Send + Sync {
    async fn ping(&self) -> AppResult<&'static str>;
}
