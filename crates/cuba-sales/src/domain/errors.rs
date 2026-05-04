#[derive(Debug, thiserror::Error)]
pub enum SalesDomainError {
    #[error("sales order not found")]
    SalesOrderNotFound,

    #[error("sales order status is invalid for this operation")]
    InvalidSalesOrderStatus,

    #[error("sales order line not found")]
    SalesOrderLineNotFound,

    #[error("sales order line number must be greater than zero")]
    InvalidLineNo,

    #[error("sales quantity must be greater than zero")]
    InvalidQuantity,

    #[error("shipment quantity exceeds open quantity")]
    ShipmentQuantityExceeded,

    #[error("customer is inactive")]
    CustomerInactive,

    #[error("material is inactive")]
    MaterialInactive,

    #[error("source bin is inactive")]
    BinInactive,

    #[error("sales order has no lines")]
    EmptySalesOrder,

    #[error("duplicated line number")]
    DuplicatedLineNo,

    #[error("no available batch for FEFO picking")]
    NoAvailableBatch,

    #[error("insufficient stock")]
    InsufficientStock,
}
