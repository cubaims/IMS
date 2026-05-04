#[derive(Debug, thiserror::Error)]
pub enum PurchaseDomainError {
    #[error("Purchase not found")]
    NotFound,
    #[error("invalid purchase state: {0}")]
    InvalidState(String),
}
