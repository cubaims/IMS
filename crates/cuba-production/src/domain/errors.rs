#[derive(Debug, thiserror::Error)]
pub enum ProductionDomainError {
    #[error("production order not found")]
    ProductionOrderNotFound,

    #[error("production order status invalid")]
    ProductionOrderStatusInvalid,

    #[error("production quantity must be greater than zero")]
    ProductionQuantityInvalid,

    #[error("production quantity exceeds remaining planned quantity")]
    ProductionQuantityExceeded,

    #[error("product variant not found")]
    ProductVariantNotFound,

    #[error("product variant inactive")]
    ProductVariantInactive,

    #[error("BOM not found")]
    BomNotFound,

    #[error("BOM inactive")]
    BomInactive,

    #[error("BOM has no components")]
    BomNoComponents,

    #[error("work center not found")]
    WorkCenterNotFound,

    #[error("work center inactive")]
    WorkCenterInactive,

    #[error("component stock shortage")]
    ComponentStockShortage,

    #[error("finished batch already exists")]
    FinishedBatchAlreadyExists,

    #[error("finished bin invalid")]
    FinishedBinInvalid,

    #[error("finished bin capacity exceeded")]
    FinishedBinCapacityExceeded,

    #[error("genealogy write failed")]
    GenealogyWriteFailed,

    #[error("production variance write failed")]
    ProductionVarianceWriteFailed,
}