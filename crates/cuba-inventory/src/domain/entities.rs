use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySummary {
    pub code: String,
    pub name: String,
    pub status: String,
}
