use serde::{Deserialize, Serialize};

use super::TraceabilityDomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchNumber(String);

impl BatchNumber {
    pub fn new(value: impl Into<String>) -> Result<Self, TraceabilityDomainError> {
        constrained_trace_target("批次号", value.into(), 30).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SerialNumber(String);

impl SerialNumber {
    pub fn new(value: impl Into<String>) -> Result<Self, TraceabilityDomainError> {
        constrained_trace_target("序列号", value.into(), 30).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn constrained_trace_target(
    target: &'static str,
    value: String,
    max_len: usize,
) -> Result<String, TraceabilityDomainError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(TraceabilityDomainError::EmptyTraceTarget { target });
    }

    if trimmed.chars().count() > max_len {
        return Err(TraceabilityDomainError::TraceTargetTooLong { target, max_len });
    }

    Ok(trimmed.to_string())
}
