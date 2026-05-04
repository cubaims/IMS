use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseSummary {
    pub code: String,
    pub name: String,
    pub status: String,
}
