pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

// 重新导出常用类型
pub use domain::*;
pub use application::*;
pub use infrastructure::PostgresAuthRepository;
pub use interface::dto::*;
pub use interface::handlers;
pub use interface::routes;
