use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl CurrentUser {
    pub fn has_permission(&self, perm: &str) -> bool {
        self.has_role("ADMIN") || self.permissions.iter().any(|p| permission_matches(p, perm))
    }

    /// 新增：检查是否拥有任意一个权限（供 require_any_permission 使用）
    pub fn has_any_permission(&self, perms: &[&str]) -> bool {
        perms.iter().any(|p| self.has_permission(p))
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r.eq_ignore_ascii_case(role))
    }
}

fn permission_matches(granted: &str, required: &str) -> bool {
    let granted = granted.trim();
    let required = required.trim();

    if granted == "*" || granted.eq_ignore_ascii_case("SYS_ALL") {
        return true;
    }

    if granted.eq_ignore_ascii_case(required) {
        return true;
    }

    if let Some(module) = granted.strip_suffix(":*") {
        return required
            .strip_prefix(module)
            .is_some_and(|suffix| suffix.starts_with(':'));
    }

    legacy_permission_matches(granted, required)
}

fn legacy_permission_matches(granted: &str, required: &str) -> bool {
    if granted.eq_ignore_ascii_case("WMS_POST_TRANSACTION") {
        return matches!(
            required,
            "inventory:post" | "inventory:transfer" | "purchase:receipt" | "sales:shipment"
        );
    }

    if granted.eq_ignore_ascii_case("WMS_COUNT") {
        return matches!(
            required,
            "inventory:read"
                | "inventory:post"
                | "inventory-count:read"
                | "inventory-count:write"
                | "inventory-count:submit"
                | "inventory-count:approve"
                | "inventory-count:post"
                | "inventory-count:close"
        );
    }

    if granted.eq_ignore_ascii_case("QM_INSPECTION") {
        return matches!(required, "quality:read" | "quality:write");
    }

    granted.eq_ignore_ascii_case("MRP_RUN") && required == "mrp:run"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(roles: Vec<&str>, permissions: Vec<&str>) -> CurrentUser {
        CurrentUser {
            user_id: Uuid::nil(),
            username: "tester".to_string(),
            full_name: None,
            email: None,
            roles: roles.into_iter().map(ToString::to_string).collect(),
            permissions: permissions.into_iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn admin_role_allows_master_data_permissions() {
        let user = user(vec!["ADMIN"], vec![]);

        assert!(user.has_permission("master-data:read"));
        assert!(user.has_permission("master-data:write"));
    }

    #[test]
    fn sys_all_allows_master_data_permissions() {
        let user = user(vec![], vec!["SYS_ALL"]);

        assert!(user.has_permission("master-data:read"));
        assert!(user.has_permission("master-data:write"));
    }

    #[test]
    fn read_permission_does_not_allow_write() {
        let user = user(vec![], vec!["master-data:read"]);

        assert!(user.has_permission("master-data:read"));
        assert!(!user.has_permission("master-data:write"));
    }

    #[test]
    fn module_wildcard_allows_master_data_permissions() {
        let user = user(vec![], vec!["master-data:*"]);

        assert!(user.has_permission("master-data:read"));
        assert!(user.has_permission("master-data:write"));
    }

    #[test]
    fn legacy_inventory_count_permission_does_not_allow_history_or_reports() {
        let user = user(vec![], vec!["WMS_COUNT"]);

        assert!(user.has_permission("inventory:read"));
        assert!(user.has_permission("inventory:post"));
        assert!(!user.has_permission("inventory:history"));
        assert!(!user.has_permission("inventory:transfer"));
        assert!(!user.has_permission("batch:history"));
        assert!(!user.has_permission("cost:map-read"));
        assert!(!user.has_permission("report:read"));
    }

    #[test]
    fn legacy_wms_post_permission_allows_order_posting_but_not_order_creation() {
        let user = user(vec![], vec!["WMS_POST_TRANSACTION"]);

        assert!(user.has_permission("inventory:post"));
        assert!(user.has_permission("inventory:transfer"));
        assert!(user.has_permission("purchase:receipt"));
        assert!(user.has_permission("sales:shipment"));
        assert!(!user.has_permission("purchase:write"));
        assert!(!user.has_permission("sales:write"));
    }

    #[test]
    fn legacy_mrp_run_permission_only_allows_run() {
        let user = user(vec![], vec!["MRP_RUN"]);

        assert!(user.has_permission("mrp:run"));
        assert!(!user.has_permission("mrp:read"));
        assert!(!user.has_permission("mrp:suggestion-confirm"));
    }

    #[test]
    fn legacy_quality_inspection_permission_does_not_allow_decisions() {
        let user = user(vec![], vec!["QM_INSPECTION"]);

        assert!(user.has_permission("quality:read"));
        assert!(user.has_permission("quality:write"));
        assert!(!user.has_permission("quality:decision"));
    }
}
