use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpSummary {
    pub code: String,
    pub name: String,
    pub status: String,
}
