use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterDataSummary {
    pub code: String,
    pub name: String,
    pub status: String,
}
