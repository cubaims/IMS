use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpCommand {
    pub request_id: Option<String>,
}
