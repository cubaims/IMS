use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingCommand {
    pub request_id: Option<String>,
}

/// 手动刷新报表物化视图命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshMaterializedViewsCommand {
    pub mode: String,
    pub concurrently: bool,
    pub remark: Option<String>,
}

impl Default for RefreshMaterializedViewsCommand {
    fn default() -> Self {
        Self {
            mode: "all".to_string(),
            concurrently: true,
            remark: None,
        }
    }
}
