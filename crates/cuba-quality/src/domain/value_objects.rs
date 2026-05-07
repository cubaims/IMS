use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{QualityError, QualityResult};

fn normalize_required(value: impl Into<String>, field: &'static str) -> QualityResult<String> {
    let value = value.into().trim().to_string();

    if value.is_empty() {
        return Err(QualityError::RequiredFieldEmpty(field));
    }

    Ok(value)
}

/// 检验批 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InspectionLotId(String);

impl InspectionLotId {
    /// 向后兼容的非校验构造。
    ///
    /// 新代码建议使用 try_new。
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 带业务校验的安全构造。
    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "inspection_lot_id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InspectionLotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 检验结果 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InspectionResultId(String);

impl InspectionResultId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "inspection_result_id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InspectionResultId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 质量通知 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QualityNotificationId(String);

impl QualityNotificationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "quality_notification_id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for QualityNotificationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 物料 ID。
///
/// 后续如果 master-data 模块也定义 MaterialId，
/// 可以把这个类型上移到 cuba-shared 或 cuba-master-data 中。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(String);

impl MaterialId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "material_id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MaterialId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 批次号。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchNumber(String);

impl BatchNumber {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "batch_number")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BatchNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 检验特性 ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InspectionCharId(String);

impl InspectionCharId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "inspection_char_id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InspectionCharId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 不良代码。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DefectCode(String);

impl DefectCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "defect_code")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DefectCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 操作人。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Operator(String);

impl Operator {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> QualityResult<Self> {
        Ok(Self(normalize_required(value, "operator")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
