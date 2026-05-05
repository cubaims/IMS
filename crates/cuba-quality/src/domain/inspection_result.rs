use crate::domain::{
    DefectCode, InspectionCharId, InspectionLotId, InspectionResultId, InspectionResultStatus,
    Operator, QualityError, QualityResult,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// 检验结果。
///
/// 一个检验批可以有多条检验结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionResult {
    pub id: InspectionResultId,

    /// 所属检验批。
    pub inspection_lot_id: InspectionLotId,

    /// 检验特性 ID。
    pub char_id: InspectionCharId,

    /// 数值型结果，例如尺寸、重量、浓度。
    pub measured_value: Option<Decimal>,

    /// 定性结果，例如 PASS / FAIL。
    pub qualitative_result: Option<InspectionResultStatus>,

    /// 下限。
    pub lower_limit: Option<Decimal>,

    /// 上限。
    pub upper_limit: Option<Decimal>,

    /// 单位。
    pub unit: Option<String>,

    /// 最终结果状态。
    pub result_status: InspectionResultStatus,

    /// 不良代码。
    pub defect_code: Option<DefectCode>,

    /// 不良数量。
    pub defect_qty: Decimal,

    pub inspector: Operator,
    pub inspected_at: OffsetDateTime,
    pub remark: Option<String>,
}

impl InspectionResult {
    /// 创建检验结果。
    pub fn create(input: CreateInspectionResult) -> QualityResult<Self> {
        if input.defect_qty < Decimal::ZERO {
            return Err(QualityError::BusinessRuleViolation(
                "不良数量不能小于 0".to_string(),
            ));
        }

        let result_status = Self::calculate_result_status(
            input.measured_value,
            input.qualitative_result,
            input.lower_limit,
            input.upper_limit,
        )?;

        if result_status == InspectionResultStatus::Fail && input.defect_code.is_none() {
            return Err(QualityError::DefectCodeRequired);
        }

        Ok(Self {
            id: input.id,
            inspection_lot_id: input.inspection_lot_id,
            char_id: input.char_id,
            measured_value: input.measured_value,
            qualitative_result: input.qualitative_result,
            lower_limit: input.lower_limit,
            upper_limit: input.upper_limit,
            unit: input.unit,
            result_status,
            defect_code: input.defect_code,
            defect_qty: input.defect_qty,
            inspector: input.inspector,
            inspected_at: input.now,
            remark: input.remark,
        })
    }

    /// 根据输入值自动计算 PASS / FAIL。
    ///
    /// 规则：
    /// - 如果传入 qualitative_result，则优先使用定性结果；
    /// - 如果传入 measured_value，则按上下限判断；
    /// - 如果都没有，则认为结果无效。
    fn calculate_result_status(
        measured_value: Option<Decimal>,
        qualitative_result: Option<InspectionResultStatus>,
        lower_limit: Option<Decimal>,
        upper_limit: Option<Decimal>,
    ) -> QualityResult<InspectionResultStatus> {
        if let Some(status) = qualitative_result {
            return Ok(status);
        }

        let Some(value) = measured_value else {
            return Err(QualityError::InspectionResultInvalid);
        };

        if let Some(lower) = lower_limit {
            if value < lower {
                return Ok(InspectionResultStatus::Fail);
            }
        }

        if let Some(upper) = upper_limit {
            if value > upper {
                return Ok(InspectionResultStatus::Fail);
            }
        }

        Ok(InspectionResultStatus::Pass)
    }

    /// 当前结果是否失败。
    pub fn is_failed(&self) -> bool {
        self.result_status == InspectionResultStatus::Fail
    }
}

/// 创建检验结果输入。
#[derive(Debug, Clone)]
pub struct CreateInspectionResult {
    pub id: InspectionResultId,
    pub inspection_lot_id: InspectionLotId,
    pub char_id: InspectionCharId,
    pub measured_value: Option<Decimal>,
    pub qualitative_result: Option<InspectionResultStatus>,
    pub lower_limit: Option<Decimal>,
    pub upper_limit: Option<Decimal>,
    pub unit: Option<String>,
    pub defect_code: Option<DefectCode>,
    pub defect_qty: Decimal,
    pub inspector: Operator,
    pub now: OffsetDateTime,
    pub remark: Option<String>,
}