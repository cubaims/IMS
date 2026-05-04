#[derive(Debug, thiserror::Error)]
pub enum ReportingDomainError {
    #[error("Reporting not found")]
    NotFound,
    #[error("invalid reporting state: {0}")]
    InvalidState(String),
}
