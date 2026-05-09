//! 审计字段统一抽象。
//!
//! 所有业务聚合根都应内嵌一个 `AuditInfo` 字段,而不是各自散落
//! `created_at`/`updated_at`/...。这样:
//! - 字段名永远一致,跨模块响应体可读性高
//! - 升级(比如加乐观锁、加 deleted_at 软删除)只动一处
//! - 持久化层负责把 DB 行映射为 `AuditInfo`,领域层不关心 DB 列名
//!
//! 设计要点:
//! - 时间用 `time::OffsetDateTime`,序列化为 RFC3339(对 PG `timestamptz` 友好)
//! - `created_by`/`updated_by` 是 `Option<Uuid>`,允许"系统初始化数据"无操作人
//! - `version` 预留做乐观锁,Phase 3 用不上但留扩展位
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::CurrentUser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditInfo {
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,

    pub created_by: Option<Uuid>,

    pub updated_by: Option<Uuid>,

    /// 乐观锁版本号。每次 `bump_for_update` +1。
    /// Phase 3 不强制,Phase 4/5 库存事务、生产订单建议启用 WHERE version = ? 的乐观更新。
    pub version: i64,
}

impl AuditInfo {
    /// 用户操作下的"新建"——把当前用户作为 created_by 与 updated_by。
    pub fn new_for_create(user: &CurrentUser) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            created_at: now,
            updated_at: now,
            created_by: Some(user.user_id),
            updated_by: Some(user.user_id),
            version: 1,
        }
    }

    /// 系统初始化场景——seed 数据、migration、worker 自动写入,
    /// 没有真实用户上下文,操作人留空。
    pub fn new_for_system() -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            created_at: now,
            updated_at: now,
            created_by: None,
            updated_by: None,
            version: 1,
        }
    }

    /// 持久化层从 DB 行还原 `AuditInfo`。
    /// 注意:`version` 如果 DB 里还没这一列,Phase 3 阶段先固定传 1。
    pub fn from_storage(
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
        created_by: Option<Uuid>,
        updated_by: Option<Uuid>,
        version: i64,
    ) -> Self {
        Self {
            created_at,
            updated_at,
            created_by,
            updated_by,
            version,
        }
    }

    /// 用户操作下的"更新"——刷新 updated_at / updated_by,version +1。
    /// `created_*` 永远不变。
    pub fn bump_for_update(&mut self, user: &CurrentUser) {
        self.updated_at = OffsetDateTime::now_utc();
        self.updated_by = Some(user.user_id);
        self.version += 1;
    }

    /// 系统场景下的更新(同上,但 updated_by 不动来源)。
    pub fn bump_for_system(&mut self) {
        self.updated_at = OffsetDateTime::now_utc();
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_user() -> CurrentUser {
        CurrentUser {
            user_id: Uuid::new_v4(),
            username: "tester".to_string(),
            full_name: None,
            email: None,
            roles: vec![],
            permissions: vec![],
        }
    }

    #[test]
    fn new_for_create_sets_creator_and_version_one() {
        let u = fake_user();
        let a = AuditInfo::new_for_create(&u);
        assert_eq!(a.created_by, Some(u.user_id));
        assert_eq!(a.updated_by, Some(u.user_id));
        assert_eq!(a.version, 1);
        assert_eq!(a.created_at, a.updated_at);
    }

    #[test]
    fn new_for_system_has_no_user() {
        let a = AuditInfo::new_for_system();
        assert!(a.created_by.is_none());
        assert!(a.updated_by.is_none());
        assert_eq!(a.version, 1);
    }

    #[test]
    fn bump_for_update_advances_version_and_updated_by() {
        let creator = fake_user();
        let editor = fake_user();
        let mut a = AuditInfo::new_for_create(&creator);
        let original_created_at = a.created_at;
        let original_created_by = a.created_by;

        // 故意 sleep 1ms 避免时间相同导致用例脆弱
        std::thread::sleep(std::time::Duration::from_millis(2));
        a.bump_for_update(&editor);

        assert_eq!(a.created_at, original_created_at, "created_at 不应改");
        assert_eq!(a.created_by, original_created_by, "created_by 不应改");
        assert!(a.updated_at > original_created_at);
        assert_eq!(a.updated_by, Some(editor.user_id));
        assert_eq!(a.version, 2);
    }

    #[test]
    fn bump_for_system_does_not_touch_updated_by() {
        let creator = fake_user();
        let mut a = AuditInfo::new_for_create(&creator);
        let original_updated_by = a.updated_by;

        a.bump_for_system();

        assert_eq!(a.updated_by, original_updated_by, "system bump 不动 updated_by");
        assert_eq!(a.version, 2);
    }
}