#[derive(Debug, thiserror::Error)]
pub enum QualityDomainError {
    #[error("Quality not found")]
    NotFound,
    #[error("invalid quality state: {0}")]
    InvalidState(String),
}
