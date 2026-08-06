//! HTTP API 层:authlib-injector 规范端点
//!
//! 本层依赖 `core`(基础设施)与 `app`(服务层),负责路由注册与请求处理。

mod auth;
mod certificates;
mod meta;
mod profiles;
mod session;

pub use auth::{authenticate, bearer_token, client_ip, invalidate, refresh, signout, validate};
pub use certificates::certificates;
pub use meta::meta;
pub use profiles::{
    batch_profiles, delete_cape, delete_skin, require_bearer, texture_file, upload_cape,
    upload_skin,
};
pub use session::{has_joined, join, profile};

use axum::Router;
use axum::middleware;
use axum::routing::{get, post, put};

use crate::app::state::AppState;

/// 组装 API 路由(authlib-injector 规范路径,直接挂根)
pub fn build_app(state: AppState) -> Router {
    let texture_auth = middleware::from_fn_with_state(state.clone(), profiles::require_bearer);
    Router::new()
        // 扩展 API:元数据 + ALI
        .route("/", get(meta::meta))
        // 认证 API
        .route("/authserver/authenticate", post(auth::authenticate))
        .route("/authserver/refresh", post(auth::refresh))
        .route("/authserver/validate", post(auth::validate))
        .route("/authserver/invalidate", post(auth::invalidate))
        .route("/authserver/signout", post(auth::signout))
        // 会话 API
        .route("/sessionserver/session/minecraft/join", post(session::join))
        .route(
            "/sessionserver/session/minecraft/hasJoined",
            get(session::has_joined),
        )
        .route(
            "/sessionserver/session/minecraft/profile/{uuid}",
            get(session::profile),
        )
        // 角色 API
        .route("/api/profiles/minecraft", post(profiles::batch_profiles))
        // 材质上传/清除(需 Bearer 认证)
        .route(
            "/api/user/profile/{uuid}/skin",
            put(profiles::upload_skin)
                .delete(profiles::delete_skin)
                .layer(texture_auth.clone()),
        )
        .route(
            "/api/user/profile/{uuid}/cape",
            put(profiles::upload_cape)
                .delete(profiles::delete_cape)
                .layer(texture_auth),
        )
        // 材质文件
        .route("/textures/{hash}", get(profiles::texture_file))
        // Minecraft 1.19+ 消息签名密钥
        .route(
            "/minecraftservices/player/certificates",
            post(certificates::certificates),
        )
        .with_state(state)
}
