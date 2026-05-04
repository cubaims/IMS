use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterDataCommand {
    pub request_id: Option<String>,
}
