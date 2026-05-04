#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MasterDataId(pub String);

impl MasterDataId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
