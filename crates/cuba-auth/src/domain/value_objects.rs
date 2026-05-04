#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthId(pub String);

impl AuthId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}
