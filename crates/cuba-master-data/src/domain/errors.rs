#[derive(Debug, thiserror::Error)]
pub enum MasterDataDomainError {
    #[error("MasterData not found")]
    NotFound,
    #[error("invalid master_data state: {0}")]
    InvalidState(String),
}
