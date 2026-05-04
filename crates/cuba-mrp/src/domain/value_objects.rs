#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MrpId(pub String);

impl MrpId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
