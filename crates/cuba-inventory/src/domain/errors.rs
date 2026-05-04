#[derive(Debug, thiserror::Error)]
pub enum InventoryDomainError {
    #[error("Inventory not found")]
    NotFound,
    #[error("invalid inventory state: {0}")]
    InvalidState(String),
}
