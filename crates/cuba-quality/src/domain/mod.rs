//! 质量模块领域层。
//!
//! 领域层只表达业务概念和业务规则，不能依赖数据库、HTTP、消息队列等基础设施。

pub mod batch_quality;
pub mod enums;
pub mod errors;
pub mod inspection_lot;
pub mod inspection_result;
pub mod quality_notification;
pub mod value_objects;

pub use batch_quality::*;
pub use enums::*;
pub use errors::*;
pub use inspection_lot::*;
pub use inspection_result::*;
pub use quality_notification::*;
pub use value_objects::*;