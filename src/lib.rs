//! yggr — 自用 Yggdrasil 认证服务端(兼容 authlib-injector 规范)

pub mod auth;
pub mod certificates;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod meta;
pub mod profiles;
pub mod seed;
pub mod session;
pub mod state;
pub mod textures;
pub mod types;

use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;

use crate::state::AppState;

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
        .route(
            "/sessionserver/session/minecraft/join",
            post(session::join),
        )
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
