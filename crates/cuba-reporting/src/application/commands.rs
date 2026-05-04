use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingCommand {
    pub request_id: Option<String>,
}
