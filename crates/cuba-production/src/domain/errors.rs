use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProductionDomainError {
    #[error("production order not found")]
    ProductionOrderNotFound,

    #[error("production order status invalid: {0}")]
    ProductionOrderStatusInvalid(String),

    #[error("production order already completed")]
    ProductionOrderAlreadyCompleted,

    #[error("production order already cancelled")]
    ProductionOrderAlreadyCancelled,

    #[error("planned quantity must be greater than zero")]
    InvalidPlannedQuantity,

    #[error("completed quantity must be greater than zero")]
    InvalidCompletedQuantity,

    #[error("completed quantity exceeds remaining planned quantity")]
    CompletedQuantityExceeded,

    #[error("product variant not found: {0}")]
    ProductVariantNotFound(String),

    #[error("product variant inactive: {0}")]
    ProductVariantInactive(String),

    #[error("BOM not found: {0}")]
    BomNotFound(String),

    #[error("BOM inactive: {0}")]
    BomInactive(String),

    #[error("BOM has no components: {0}")]
    BomNoComponents(String),

    #[error("work center not found: {0}")]
    WorkCenterNotFound(String),

    #[error("work center inactive: {0}")]
    WorkCenterInactive(String),

    #[error("component stock shortage: {0}")]
    ComponentStockShortage(String),

    #[error("finished batch already exists: {0}")]
    FinishedBatchAlreadyExists(String),

    #[error("finished bin invalid: {0}")]
    FinishedBinInvalid(String),

    #[error("genealogy write failed")]
    GenealogyWriteFailed,

    #[error("production variance write failed")]
    ProductionVarianceWriteFailed,
}