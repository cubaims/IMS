use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySummary {
    pub code: String,
    pub name: String,
    pub status: String,
}
