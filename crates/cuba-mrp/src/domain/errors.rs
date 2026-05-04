#[derive(Debug, thiserror::Error)]
pub enum MrpDomainError {
    #[error("Mrp not found")]
    NotFound,
    #[error("invalid mrp state: {0}")]
    InvalidState(String),
}
