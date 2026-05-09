//! 主数据领域错误。
//!
//! 设计:
//! - 字段级错误(Invalid*Id / *TooLong / NameCannotBeEmpty 等)→ `AppError::Validation`,
//!   前端按 message 展示,不分流。
//! - 业务级错误(MaterialNotFound / BomCycleDetected 等)→ `AppError::Business { code }`,
//!   code 与 Phase 3 §十一 错误码清单 1:1 对应,前端按 code 分流。
//!
//! 实际映射在 `application/error_mapping.rs` 的 `From<MasterDataDomainError> for AppError`。

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MasterDataDomainError {
    // ============================================================
    // 字段校验类(→ AppError::Validation,无 business code)
    // ============================================================

    #[error("名称不能为空")]
    NameCannotBeEmpty,

    #[error("金额不能为负数")]
    AmountCannotBeNegative,

    #[error("数量必须大于 0")]
    QuantityMustBeGreaterThanZero,

    // ----- ID / Code 格式 -----
    #[error("物料编码不能为空")]
    InvalidMaterialId,
    #[error("物料编码长度超限(最长 20)")]
    MaterialIdTooLong,

    #[error("货位编码不能为空")]
    InvalidBinCode,
    #[error("货位编码长度超限(最长 20)")]
    BinCodeTooLong,

    #[error("供应商编码不能为空")]
    InvalidSupplierId,
    #[error("供应商编码长度超限(最长 20)")]
    SupplierIdTooLong,

    #[error("客户编码不能为空")]
    InvalidCustomerId,
    #[error("客户编码长度超限(最长 20)")]
    CustomerIdTooLong,

    #[error("BOM 编码不能为空")]
    InvalidBomId,
    #[error("BOM 编码长度超限(最长 30)")]
    BomIdTooLong,

    #[error("产品变体编码不能为空")]
    InvalidVariantCode,
    #[error("产品变体编码长度超限(最长 20)")]
    VariantCodeTooLong,

    #[error("工作中心编码不能为空")]
    InvalidWorkCenterId,
    #[error("工作中心编码长度超限(最长 20)")]
    WorkCenterIdTooLong,

    #[error("检验特性编码不能为空")]
    InvalidInspectionCharId,
    #[error("检验特性编码长度超限(最长 30)")]
    InspectionCharIdTooLong,

    #[error("不良代码不能为空")]
    InvalidDefectCode,
    #[error("不良代码长度超限(最长 20)")]
    DefectCodeTooLong,

    // ----- 业务字段约束(仍归 Validation) -----
    #[error("货位容量不能为负数")]
    CapacityCannotBeNegative,

    #[error("货位容量不能小于当前占用量")]
    CapacityCannotBeLessThanOccupied,

    #[error("货位容量值不合法")]
    BinCapacityInvalid,

    #[error("BOM 组件不能引用自身")]
    BomComponentCannotReferenceItself,

    #[error("检验特性上下限不合法(上限不能小于下限)")]
    InspectionLimitInvalid,

    // ============================================================
    // 业务规则违反(→ AppError::Business { code })
    // 一一对应 Phase 3 §十一 错误码清单
    // ============================================================

    // ----- Material -----
    #[error("物料不存在")]
    MaterialNotFound,
    #[error("物料已存在,不能重复创建")]
    MaterialAlreadyExists,
    #[error("物料已停用,不能用于新业务")]
    MaterialInactive,
    #[error("物料还有库存,不允许此操作")]
    MaterialHasStock,

    // ----- Bin -----
    #[error("货位不存在")]
    BinNotFound,
    #[error("货位已存在,不能重复创建")]
    BinAlreadyExists,
    #[error("货位已停用,不能入库")]
    BinInactive,
    #[error("货位还有库存,不允许此操作")]
    BinHasStock,

    // ----- Supplier -----
    #[error("供应商不存在")]
    SupplierNotFound,
    #[error("供应商已存在,不能重复创建")]
    SupplierAlreadyExists,
    #[error("供应商已停用,不能用于新业务")]
    SupplierInactive,
    #[error("已存在主供应商,不能重复设置")]
    PrimarySupplierAlreadyExists,

    // ----- Customer -----
    #[error("客户不存在")]
    CustomerNotFound,
    #[error("客户已存在,不能重复创建")]
    CustomerAlreadyExists,
    #[error("客户已停用,不能用于新业务")]
    CustomerInactive,

    // ----- ProductVariant -----
    #[error("产品变体不存在")]
    VariantNotFound,
    #[error("产品变体已存在,不能重复创建")]
    VariantAlreadyExists,
    #[error("产品变体已停用,不能用于新业务")]
    VariantInactive,

    // ----- BOM -----
    #[error("BOM 不存在")]
    BomNotFound,
    #[error("BOM 已存在,不能重复创建")]
    BomAlreadyExists,
    #[error("BOM 组件不存在")]
    BomComponentNotFound,
    #[error("BOM 组件重复(同一物料已存在于该 BOM)")]
    BomComponentDuplicated,
    #[error("BOM 不能引用自身(parent = component)")]
    BomSelfReference,
    #[error("BOM 存在循环引用")]
    BomCycleDetected,
    #[error("BOM 至少需要一个组件,无法启用")]
    BomNoComponents,

    // ----- WorkCenter -----
    #[error("工作中心不存在")]
    WorkCenterNotFound,
    #[error("工作中心已存在,不能重复创建")]
    WorkCenterAlreadyExists,
    #[error("工作中心已停用,不能用于新业务")]
    WorkCenterInactive,

    // ----- InspectionChar -----
    #[error("检验特性不存在")]
    InspectionCharNotFound,
    #[error("检验特性已存在,不能重复创建")]
    InspectionCharAlreadyExists,

    // ----- DefectCode -----
    #[error("不良代码不存在")]
    DefectCodeNotFound,
    #[error("不良代码已存在,不能重复创建")]
    DefectCodeAlreadyExists,
    #[error("不良代码已停用,不能用于新业务")]
    DefectCodeInactive,
}

impl MasterDataDomainError {
    /// 业务码(与 Phase 3 §十一 1:1 对应)。
    /// 字段校验类返回 None,统一走 VALIDATION_ERROR。
    pub fn business_code(&self) -> Option<&'static str> {
        Some(match self {
            // Material
            Self::MaterialNotFound => "MATERIAL_NOT_FOUND",
            Self::MaterialAlreadyExists => "MATERIAL_ALREADY_EXISTS",
            Self::MaterialInactive => "MATERIAL_INACTIVE",
            Self::MaterialHasStock => "MATERIAL_HAS_STOCK",
            // Bin
            Self::BinNotFound => "BIN_NOT_FOUND",
            Self::BinAlreadyExists => "BIN_ALREADY_EXISTS",
            Self::BinInactive => "BIN_INACTIVE",
            Self::BinHasStock => "BIN_HAS_STOCK",
            // Supplier
            Self::SupplierNotFound => "SUPPLIER_NOT_FOUND",
            Self::SupplierAlreadyExists => "SUPPLIER_ALREADY_EXISTS",
            Self::SupplierInactive => "SUPPLIER_INACTIVE",
            Self::PrimarySupplierAlreadyExists => "PRIMARY_SUPPLIER_ALREADY_EXISTS",
            // Customer
            Self::CustomerNotFound => "CUSTOMER_NOT_FOUND",
            Self::CustomerAlreadyExists => "CUSTOMER_ALREADY_EXISTS",
            Self::CustomerInactive => "CUSTOMER_INACTIVE",
            // Variant
            Self::VariantNotFound => "VARIANT_NOT_FOUND",
            Self::VariantAlreadyExists => "VARIANT_ALREADY_EXISTS",
            Self::VariantInactive => "VARIANT_INACTIVE",
            // BOM
            Self::BomNotFound => "BOM_NOT_FOUND",
            Self::BomAlreadyExists => "BOM_ALREADY_EXISTS",
            Self::BomComponentNotFound => "BOM_COMPONENT_NOT_FOUND",
            Self::BomComponentDuplicated => "BOM_COMPONENT_DUPLICATED",
            Self::BomSelfReference => "BOM_SELF_REFERENCE",
            Self::BomCycleDetected => "BOM_CYCLE_DETECTED",
            Self::BomNoComponents => "BOM_NO_COMPONENTS",
            // WorkCenter
            Self::WorkCenterNotFound => "WORK_CENTER_NOT_FOUND",
            Self::WorkCenterAlreadyExists => "WORK_CENTER_ALREADY_EXISTS",
            Self::WorkCenterInactive => "WORK_CENTER_INACTIVE",
            // InspectionChar
            Self::InspectionCharNotFound => "INSPECTION_CHAR_NOT_FOUND",
            Self::InspectionCharAlreadyExists => "INSPECTION_CHAR_ALREADY_EXISTS",
            // DefectCode
            Self::DefectCodeNotFound => "DEFECT_CODE_NOT_FOUND",
            Self::DefectCodeAlreadyExists => "DEFECT_CODE_ALREADY_EXISTS",
            Self::DefectCodeInactive => "DEFECT_CODE_INACTIVE",
            // 字段校验类
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_errors_have_codes() {
        assert_eq!(
            MasterDataDomainError::MaterialNotFound.business_code(),
            Some("MATERIAL_NOT_FOUND")
        );
        assert_eq!(
            MasterDataDomainError::BomCycleDetected.business_code(),
            Some("BOM_CYCLE_DETECTED")
        );
        assert_eq!(
            MasterDataDomainError::PrimarySupplierAlreadyExists.business_code(),
            Some("PRIMARY_SUPPLIER_ALREADY_EXISTS")
        );
    }

    #[test]
    fn validation_errors_have_no_business_code() {
        assert_eq!(MasterDataDomainError::NameCannotBeEmpty.business_code(), None);
        assert_eq!(MasterDataDomainError::MaterialIdTooLong.business_code(), None);
        assert_eq!(MasterDataDomainError::InspectionLimitInvalid.business_code(), None);
        assert_eq!(MasterDataDomainError::CapacityCannotBeNegative.business_code(), None);
    }

    #[test]
    fn display_messages_are_chinese() {
        assert!(MasterDataDomainError::MaterialNotFound.to_string().contains("物料"));
        assert!(MasterDataDomainError::BomCycleDetected.to_string().contains("循环"));
    }
}