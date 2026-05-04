use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesSummary {
    pub code: String,
    pub name: String,
    pub status: String,
}
