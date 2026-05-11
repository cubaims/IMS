//! BOM 聚合根。
//!
//! 把 `BomHeader` 与它的 `BomComponent` 列表组合在一起,让原本散落在
//! SQL 里的"BOM 至少一个组件才能启用 / 组件不能重复 / 组件数量 > 0 /
//! 组件不能引用自身"这些跨行规则在 Rust 域内被强制,不再依赖
//! PostgreSQL 约束兜底。
//!
//! ## 当前 PR 范围
//!
//! 只在 domain 层引入聚合 + 单元测试,**不**修改 application/infrastructure。
//! 现有的 `services.rs::add_component`、`postgres.rs::add_component` 路径保持不变,
//! 51 个既有测试照常通过。后续 PR 再做"加载聚合 → 应用变更 → 持久化"
//! 这条新数据路径,与旧路径并存一段时间逐步迁移。
//!
//! ## 不在聚合里强制的规则
//!
//! - **循环引用检测**:跨聚合规则(要看其他 active BOM),
//!   留给 domain service / repository 层。
//! - **`parent_material_id` / `component_material_id` 在 `mdm_materials` 中存在**:
//!   FK 类约束,留给 repository 层。

use super::super::{BomComponent, BomHeader, BomId, BomStatus, MasterDataDomainError, MaterialId};

#[derive(Debug, Clone)]
pub struct Bom {
    header: BomHeader,
    components: Vec<BomComponent>,
}

impl Bom {
    /// 用一个新建的 header 创建空 BOM(草稿态、零组件)。
    pub fn new(header: BomHeader) -> Self {
        Self {
            header,
            components: Vec::new(),
        }
    }

    /// 从持久层重建聚合。
    ///
    /// 由 repository 调用,**前置条件**(repo 保证):
    /// - 所有 `components[i].bom_id == header.bom_id`
    /// - 入参 components 已按 `(parent, component)` 边在 DB 唯一键约束下去重
    pub fn from_storage(header: BomHeader, components: Vec<BomComponent>) -> Self {
        Self { header, components }
    }

    pub fn id(&self) -> &BomId {
        &self.header.bom_id
    }

    pub fn header(&self) -> &BomHeader {
        &self.header
    }

    pub fn components(&self) -> &[BomComponent] {
        &self.components
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// 添加一个组件。
    ///
    /// 不变式:
    /// - 同一组件物料在一个 BOM 中只能出现一次 → `BomComponentDuplicated`
    ///
    /// 已经由 `BomComponent::new` 把守的不变式(此处不重复检查):
    /// - `parent_material_id != component_material_id`
    /// - `quantity > 0`
    /// - `unit` 非空
    pub fn add_component(&mut self, component: BomComponent) -> Result<(), MasterDataDomainError> {
        let dup = self
            .components
            .iter()
            .any(|c| c.component_material_id == component.component_material_id);
        if dup {
            return Err(MasterDataDomainError::BomComponentDuplicated);
        }
        self.components.push(component);
        Ok(())
    }

    /// 按 `component_material_id` 移除组件(找不到 → `BomComponentNotFound`)。
    pub fn remove_component(
        &mut self,
        component_material_id: &MaterialId,
    ) -> Result<(), MasterDataDomainError> {
        let before = self.components.len();
        self.components
            .retain(|c| c.component_material_id != *component_material_id);
        if self.components.len() == before {
            return Err(MasterDataDomainError::BomComponentNotFound);
        }
        Ok(())
    }

    /// 启用 BOM。
    ///
    /// 不变式:至少一个组件 → 否则 `BomNoComponents`。
    /// 已是 Active → idempotent。
    /// 循环引用不在这里查(跨聚合)。
    pub fn activate(&mut self) -> Result<(), MasterDataDomainError> {
        if self.components.is_empty() {
            return Err(MasterDataDomainError::BomNoComponents);
        }
        if matches!(self.header.status, BomStatus::Active) {
            return Ok(());
        }
        self.header.status = BomStatus::Active;
        self.header.is_active = true;
        Ok(())
    }

    /// 停用 BOM。idempotent。
    pub fn deactivate(&mut self) {
        self.header.status = BomStatus::Inactive;
        self.header.is_active = false;
    }

    /// 修改 BOM 名称。空白 / 全空格 → `NameCannotBeEmpty`。
    pub fn rename(&mut self, new_name: impl Into<String>) -> Result<(), MasterDataDomainError> {
        let new_name = new_name.into().trim().to_string();
        if new_name.is_empty() {
            return Err(MasterDataDomainError::NameCannotBeEmpty);
        }
        self.header.bom_name = new_name;
        Ok(())
    }
}

// ============================================================
// 单元测试 — 聚合层不变式,完全 in-memory,不依赖 DB
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).expect("test fixture should be valid")
    }
    fn mid(s: &str) -> MaterialId {
        MaterialId::new(s).expect("test fixture should be valid")
    }
    fn fresh_bom() -> Bom {
        let header = BomHeader::new(
            BomId::new("BOM01").expect("test fixture should be valid"),
            "Top BOM",
            mid("M_PARENT"),
            "v1",
        )
        .expect("test fixture should be valid");
        Bom::new(header)
    }
    fn comp(parent: &str, child: &str, qty: &str) -> BomComponent {
        BomComponent::new(
            BomId::new("BOM01").expect("test fixture should be valid"),
            mid(parent),
            mid(child),
            d(qty),
            "EA",
        )
        .expect("test fixture should be valid")
    }

    #[test]
    fn new_bom_is_empty_and_draft() {
        let bom = fresh_bom();
        assert_eq!(bom.component_count(), 0);
        assert!(matches!(bom.header().status, BomStatus::Draft));
    }

    #[test]
    fn cannot_activate_empty_bom() {
        // 计划 §五.7:BOM 至少需要一个组件才能启用
        let mut bom = fresh_bom();
        assert!(matches!(
            bom.activate(),
            Err(MasterDataDomainError::BomNoComponents)
        ));
        // 状态保持 Draft
        assert!(matches!(bom.header().status, BomStatus::Draft));
    }

    #[test]
    fn add_component_success() {
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        assert_eq!(bom.component_count(), 1);
    }

    #[test]
    fn add_duplicate_edge_fails() {
        // 计划 §五.7:同一 BOM 内 (parent, component) 边不能重复
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        let r = bom.add_component(comp("M_PARENT", "C1", "5"));
        assert!(matches!(
            r,
            Err(MasterDataDomainError::BomComponentDuplicated)
        ));
        assert_eq!(bom.component_count(), 1, "失败时不应改动列表");
    }

    #[test]
    fn same_component_under_different_parent_is_rejected() {
        // 当前 DB 唯一约束是 (bom_id, component_material_id),没有 line_no。
        // 因此同一组件物料不能在同一个 BOM 中重复出现,即使 parent 不同。
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "SUB", "1"))
            .expect("test fixture should be valid");
        bom.add_component(comp("SUB", "C1", "2"))
            .expect("test fixture should be valid");
        let r = bom.add_component(comp("M_PARENT", "C1", "3"));
        assert!(matches!(
            r,
            Err(MasterDataDomainError::BomComponentDuplicated)
        ));
        assert_eq!(bom.component_count(), 2);
    }

    #[test]
    fn activate_with_components_succeeds() {
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        bom.activate().expect("test fixture should be valid");
        assert!(matches!(bom.header().status, BomStatus::Active));
        assert!(bom.header().is_active);
    }

    #[test]
    fn activate_is_idempotent() {
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        bom.activate().expect("test fixture should be valid");
        bom.activate().expect("test fixture should be valid");
        assert!(matches!(bom.header().status, BomStatus::Active));
    }

    #[test]
    fn deactivate_clears_active_flags() {
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        bom.activate().expect("test fixture should be valid");
        bom.deactivate();
        assert!(matches!(bom.header().status, BomStatus::Inactive));
        assert!(!bom.header().is_active);
    }

    #[test]
    fn remove_component_success() {
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        bom.add_component(comp("M_PARENT", "C2", "3"))
            .expect("test fixture should be valid");
        bom.remove_component(&mid("C1"))
            .expect("test fixture should be valid");
        assert_eq!(bom.component_count(), 1);
        assert_eq!(bom.components()[0].component_material_id, mid("C2"));
    }

    #[test]
    fn remove_unknown_component_fails() {
        let mut bom = fresh_bom();
        bom.add_component(comp("M_PARENT", "C1", "2"))
            .expect("test fixture should be valid");
        let r = bom.remove_component(&mid("DOES_NOT_EXIST"));
        assert!(matches!(
            r,
            Err(MasterDataDomainError::BomComponentNotFound)
        ));
        assert_eq!(bom.component_count(), 1);
    }

    #[test]
    fn rename_rejects_blank() {
        let mut bom = fresh_bom();
        let r = bom.rename("   ");
        assert!(matches!(r, Err(MasterDataDomainError::NameCannotBeEmpty)));
    }

    #[test]
    fn rename_trims_and_updates() {
        let mut bom = fresh_bom();
        bom.rename("  New Name  ")
            .expect("test fixture should be valid");
        assert_eq!(bom.header().bom_name, "New Name");
    }

    #[test]
    fn self_reference_blocked_at_component_constructor_layer() {
        // 文档化:聚合假设入参 component 已经被 BomComponent::new 把过自引用关。
        // 即聚合不重复检查,只测试入口处的把关确实在工作。
        let r = BomComponent::new(
            BomId::new("BOM01").expect("test fixture should be valid"),
            mid("X"),
            mid("X"),
            d("1"),
            "EA",
        );
        assert!(matches!(
            r,
            Err(MasterDataDomainError::BomComponentCannotReferenceItself)
        ));
    }
}
