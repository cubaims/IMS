#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProductionId(pub String);

impl ProductionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
