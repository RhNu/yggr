//! 基础设施层:配置、密码学、错误、公共类型与数据层
//!
//! 本层不依赖任何业务模块,是 `app`(服务层)与 `api`(HTTP 层)的公共底座。

pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod types;

pub use config::*;
pub use crypto::*;
pub use db::*;
pub use error::*;
pub use types::*;
