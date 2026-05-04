#[derive(Debug, thiserror::Error)]
pub enum AuthDomainError {
    #[error("Auth not found")]
    NotFound,
    #[error("invalid auth state: {0}")]
    InvalidState(String),
}
