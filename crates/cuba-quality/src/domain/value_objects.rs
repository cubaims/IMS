#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualityId(pub String);

impl QualityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
