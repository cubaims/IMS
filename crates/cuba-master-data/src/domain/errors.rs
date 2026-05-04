use thiserror::Error;

#[derive(Debug, Error)]
pub enum MasterDataDomainError {
    #[error("invalid material id")]
    InvalidMaterialId,

    #[error("material id too long")]
    MaterialIdTooLong,

    #[error("invalid bin code")]
    InvalidBinCode,

    #[error("bin code too long")]
    BinCodeTooLong,

    #[error("invalid supplier id")]
    InvalidSupplierId,

    #[error("supplier id too long")]
    SupplierIdTooLong,

    #[error("invalid customer id")]
    InvalidCustomerId,

    #[error("customer id too long")]
    CustomerIdTooLong,

    #[error("invalid bom id")]
    InvalidBomId,

    #[error("bom id too long")]
    BomIdTooLong,

    #[error("invalid variant code")]
    InvalidVariantCode,

    #[error("variant code too long")]
    VariantCodeTooLong,

    #[error("invalid work center id")]
    InvalidWorkCenterId,

    #[error("work center id too long")]
    WorkCenterIdTooLong,

    #[error("invalid inspection char id")]
    InvalidInspectionCharId,

    #[error("inspection char id too long")]
    InspectionCharIdTooLong,

    #[error("invalid defect code")]
    InvalidDefectCode,

    #[error("defect code too long")]
    DefectCodeTooLong,

    #[error("name cannot be empty")]
    NameCannotBeEmpty,

    #[error("quantity must be greater than zero")]
    QuantityMustBeGreaterThanZero,

    #[error("amount cannot be negative")]
    AmountCannotBeNegative,

    #[error("capacity cannot be negative")]
    CapacityCannotBeNegative,

    #[error("capacity cannot be less than occupied quantity")]
    CapacityCannotBeLessThanOccupied,

    #[error("bom component cannot reference itself")]
    BomComponentCannotReferenceItself,

    #[error("inspection upper limit cannot be less than lower limit")]
    InspectionLimitInvalid,
}