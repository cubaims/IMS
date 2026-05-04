#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InventoryId(pub String);

impl InventoryId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
