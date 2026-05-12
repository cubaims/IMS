pub mod commands;
pub mod ports;
pub mod services;
pub mod use_cases;

pub use commands::*;
pub use ports::*;
pub use services::*;
pub use use_cases::*;

use cuba_shared::AppError;

use crate::domain::QualityError;

impl From<QualityError> for AppError {
    fn from(err: QualityError) -> Self {
        match err {
            QualityError::InspectionLotNotFound => AppError::NotFound(err.to_string()),
            QualityError::InspectionResultNotFound => AppError::NotFound(err.to_string()),
            QualityError::QualityNotificationNotFound => AppError::NotFound(err.to_string()),
            QualityError::InspectionCharNotFound => {
                AppError::business("INSPECTION_CHAR_NOT_FOUND", err.to_string())
            }
            QualityError::DefectCodeNotFound => {
                AppError::business("DEFECT_CODE_NOT_FOUND", err.to_string())
            }
            QualityError::RequiredFieldEmpty(_)
            | QualityError::QuantityMustBePositive
            | QualityError::SampleQtyExceeded
            | QualityError::InspectionResultInvalid
            | QualityError::QualityDecisionInvalid
            | QualityError::BatchQualityStatusInvalid => AppError::Validation(err.to_string()),
            QualityError::InspectionLotAlreadyExists => {
                AppError::business("INSPECTION_LOT_ALREADY_EXISTS", err.to_string())
            }
            QualityError::InspectionCharInactive => {
                AppError::business("INSPECTION_CHAR_INACTIVE", err.to_string())
            }
            QualityError::DefectCodeInactive => {
                AppError::business("DEFECT_CODE_INACTIVE", err.to_string())
            }
            QualityError::BusinessRuleViolation(message) if message == "质量模块数据库操作失败" => {
                AppError::Internal(message)
            }
            other => AppError::business("QUALITY_RULE_VIOLATION", other.to_string()),
        }
    }
}
