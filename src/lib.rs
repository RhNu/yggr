//! yggr — 自用 Yggdrasil 认证服务端(兼容 authlib-injector 规范)
//!
//! 模块分层(依赖单向 api → app → core):
//! - [core](core):基础设施(配置、密码学、错误、公共类型、数据层)
//! - [app](app):应用服务(共享状态、材质系统、种子初始化)
//! - [api](api):HTTP 处理器与路由组装

pub mod api;
pub mod app;
pub mod core;

// 兼容路径(迁移期):新代码建议使用 yggr::core::* / yggr::app::* / yggr::api::*
pub use api::build_app;
pub use app::{seed, state, textures};
pub use core::{config, crypto, db, error, types};
