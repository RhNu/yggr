//! 应用服务层:共享状态、材质系统、种子初始化
//!
//! 本层依赖 `core`(基础设施),被 `api`(HTTP 层)与二进制入口使用。

pub mod seed;
pub mod state;
pub mod textures;

pub use seed::apply_seed;
pub use state::{AppState, JoinRecord, RateLimiter};
