#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PurchaseId(pub String);

impl PurchaseId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
