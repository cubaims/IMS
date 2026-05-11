#[derive(Debug, thiserror::Error)]
pub enum TraceabilityDomainError {
    #[error("{target}不能为空")]
    EmptyTraceTarget { target: &'static str },

    #[error("{target}长度不能超过{max_len}个字符")]
    TraceTargetTooLong {
        target: &'static str,
        max_len: usize,
    },
}
