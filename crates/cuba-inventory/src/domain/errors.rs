use thiserror::Error;

#[derive(Debug, Error)]
pub enum InventoryDomainError {
    #[error("material id cannot be empty")]
    EmptyMaterialId,

    #[error("bin code cannot be empty")]
    EmptyBinCode,

    #[error("batch number cannot be empty")]
    EmptyBatchNumber,

    #[error("transaction id cannot be empty")]
    EmptyTransactionId,

    #[error("quantity must be greater than zero")]
    InvalidQuantity,

    #[error("unit price must be greater than zero")]
    InvalidUnitPrice,

    #[error("from_bin and to_bin cannot be the same")]
    SameSourceAndTargetBin,

    #[error("movement type {0} is not supported")]
    UnsupportedMovementType(String),

    #[error("movement type {0} requires from_bin")]
    FromBinRequired(String),

    #[error("movement type {0} requires to_bin")]
    ToBinRequired(String),

    #[error("movement type {0} must not include from_bin")]
    FromBinMustBeEmpty(String),

    #[error("movement type {0} must not include to_bin")]
    ToBinMustBeEmpty(String),

    #[error("reference_doc or remark is required for manual inventory posting")]
    ReferenceDocOrRemarkRequired,

    #[error("quality status {0} cannot be issued from stock")]
    InvalidOutboundQualityStatus(String),

    #[error("batch number is required for this inventory operation")]
    BatchRequired,

    #[error("quality status {0} is invalid")]
    InvalidQualityStatus(String),
}
