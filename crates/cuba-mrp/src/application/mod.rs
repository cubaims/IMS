pub mod ports;

pub use ports::*;

impl From<crate::domain::MrpError> for cuba_shared::AppError {
    fn from(err: crate::domain::MrpError) -> Self {
        match err {
            crate::domain::MrpError::MrpRunNotFound => cuba_shared::AppError::Business {
                code: "MRP_RUN_NOT_FOUND",
                message: err.to_string(),
            },

            crate::domain::MrpError::MrpSuggestionNotFound => cuba_shared::AppError::Business {
                code: "MRP_SUGGESTION_NOT_FOUND",
                message: err.to_string(),
            },

            crate::domain::MrpError::MrpSuggestionStatusInvalid => {
                cuba_shared::AppError::Business {
                    code: "MRP_SUGGESTION_STATUS_INVALID",
                    message: err.to_string(),
                }
            }

            crate::domain::MrpError::MaterialNotFoundOrInactive => {
                cuba_shared::AppError::Business {
                    code: "MRP_MATERIAL_NOT_FOUND_OR_INACTIVE",
                    message: err.to_string(),
                }
            }

            crate::domain::MrpError::ProductVariantNotFound => cuba_shared::AppError::Business {
                code: "MRP_VARIANT_NOT_FOUND",
                message: err.to_string(),
            },

            crate::domain::MrpError::ProductVariantRequired => cuba_shared::AppError::Business {
                code: "MRP_VARIANT_REQUIRED",
                message: err.to_string(),
            },

            crate::domain::MrpError::DemandQtyMustBePositive
            | crate::domain::MrpError::DemandDateRequired
            | crate::domain::MrpError::DemandDateBeforeToday
            | crate::domain::MrpError::RequiredFieldEmpty(_) => cuba_shared::AppError::Business {
                code: "MRP_DEMAND_INVALID",
                message: err.to_string(),
            },

            crate::domain::MrpError::MrpRunFailed => cuba_shared::AppError::Business {
                code: "MRP_RUN_FAILED",
                message: err.to_string(),
            },

            crate::domain::MrpError::BusinessRuleViolation(message) => {
                cuba_shared::AppError::Business {
                    code: "MRP_BUSINESS_RULE_VIOLATION",
                    message,
                }
            }
        }
    }
}
