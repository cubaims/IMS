#[derive(Debug, thiserror::Error)]
pub enum ProductionDomainError {
    #[error("Production not found")]
    NotFound,
    #[error("invalid production state: {0}")]
    InvalidState(String),
}
