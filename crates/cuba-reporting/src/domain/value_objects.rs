#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReportingId(pub String);

impl ReportingId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
