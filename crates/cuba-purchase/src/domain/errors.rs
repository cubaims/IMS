#[derive(Debug, thiserror::Error)]
pub enum PurchaseDomainError {
    #[error("purchase order not found")]
    PurchaseOrderNotFound,

    #[error("purchase order status is invalid for this operation")]
    InvalidPurchaseOrderStatus,

    #[error("purchase order line not found")]
    PurchaseOrderLineNotFound,

    #[error("purchase order line number must be greater than zero")]
    InvalidLineNo,

    #[error("purchase quantity must be greater than zero")]
    InvalidQuantity,

    #[error("receipt quantity exceeds open quantity")]
    ReceiptQuantityExceeded,

    #[error("supplier is inactive")]
    SupplierInactive,

    #[error("material is inactive")]
    MaterialInactive,

    #[error("target bin is inactive")]
    BinInactive,

    #[error("purchase order has no lines")]
    EmptyPurchaseOrder,

    #[error("duplicated line number")]
    DuplicatedLineNo,
}
