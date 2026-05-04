#[derive(Debug, thiserror::Error)]
pub enum SalesDomainError {
    #[error("Sales not found")]
    NotFound,
    #[error("invalid sales state: {0}")]
    InvalidState(String),
}
