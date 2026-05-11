//! 主数据领域错误 → AppError 映射。
//!
//! 边界:domain 层不知道 HTTP / AppError 的存在,所以这个 From 实现必须放在
//! application 层 —— 这里同时知道 `domain::MasterDataDomainError` 与
//! `cuba_shared::AppError`。
//!
//! 完成后,service 层可以直接 `?` 传播领域错误,不需要 `.map_err(...)`。

use cuba_shared::AppError;

use crate::domain::MasterDataDomainError;

impl From<MasterDataDomainError> for AppError {
    fn from(err: MasterDataDomainError) -> Self {
        match err.business_code() {
            // 业务级错误 → AppError::Business { code }
            Some(code) => AppError::business(code, err.to_string()),
            // 字段校验类 → AppError::Validation(message)
            None => AppError::Validation(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_not_found_maps_to_business_with_code() {
        let app_err: AppError = MasterDataDomainError::MaterialNotFound.into();
        match app_err {
            AppError::Business { code, message } => {
                assert_eq!(code, "MATERIAL_NOT_FOUND");
                assert!(message.contains("物料"));
            }
            other => panic!("expected Business, got {:?}", other),
        }
    }

    #[test]
    fn name_empty_maps_to_validation() {
        let app_err: AppError = MasterDataDomainError::NameCannotBeEmpty.into();
        assert!(matches!(app_err, AppError::Validation(_)));
    }

    #[test]
    fn id_too_long_maps_to_validation() {
        let app_err: AppError = MasterDataDomainError::MaterialIdTooLong.into();
        assert!(matches!(app_err, AppError::Validation(_)));
    }

    #[test]
    fn bom_cycle_detected_maps_to_business() {
        let app_err: AppError = MasterDataDomainError::BomCycleDetected.into();
        match app_err {
            AppError::Business { code, message } => {
                assert_eq!(code, "BOM_CYCLE_DETECTED");
                assert!(message.contains("循环"));
            }
            other => panic!("expected Business, got {:?}", other),
        }
    }

    #[test]
    fn primary_supplier_conflict_maps_to_business() {
        let app_err: AppError = MasterDataDomainError::PrimarySupplierAlreadyExists.into();
        match app_err {
            AppError::Business { code, .. } => {
                assert_eq!(code, "PRIMARY_SUPPLIER_ALREADY_EXISTS");
            }
            other => panic!("expected Business, got {:?}", other),
        }
    }

    #[test]
    fn inspection_limit_invalid_maps_to_business_code() {
        let app_err: AppError = MasterDataDomainError::InspectionLimitInvalid.into();
        match app_err {
            AppError::Business { code, message } => {
                assert_eq!(code, "INSPECTION_LIMIT_INVALID");
                assert!(message.contains("上下限"));
            }
            other => panic!("expected Business, got {:?}", other),
        }
    }

    #[test]
    fn bin_capacity_invalid_maps_to_business_code() {
        let app_err: AppError = MasterDataDomainError::BinCapacityInvalid.into();
        match app_err {
            AppError::Business { code, message } => {
                assert_eq!(code, "BIN_CAPACITY_INVALID");
                assert!(message.contains("容量"));
            }
            other => panic!("expected Business, got {:?}", other),
        }
    }
}
