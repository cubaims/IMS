#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SalesId(pub String);

impl SalesId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
