//! 聚合根模块。
//!
//! 每个聚合根对应一个文件:聚合根 + 它的 children 实体 + 自洽的不变式。
//! 跨聚合规则不放这里,留给 application 层的 domain service。

pub mod bom;

pub use bom::Bom;
