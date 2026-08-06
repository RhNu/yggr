//! HTTP API 层:authlib-injector 规范端点
//!
//! 本层依赖 `core`(基础设施)与 `app`(服务层),负责路由注册与请求处理。
//! 所有 Yggdrasil 规范端点挂载在 `/service` 下,根路径 `/` 留给管理 API 与前端。

mod auth;
mod certificates;
mod meta;
mod profiles;
mod session;

pub use auth::{
    TokenStatus, authenticate, bearer_token, client_ip, invalidate, refresh, signout, validate,
};
pub use certificates::certificates;
pub use meta::meta;
pub use profiles::{
    batch_profiles, delete_cape, delete_skin, legacy_skin, require_bearer, texture_file,
    upload_cape, upload_skin,
};
pub use session::{has_joined, join, profile};

use axum::Router;
use axum::http::HeaderName;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};

use crate::app::state::AppState;

/// ALI 头值:指向 /service
const ALI_LOCATION: &str = "/service";

/// 根路径 GET / - 返回空 200 + ALI 头
async fn root() -> impl IntoResponse {
    [(
        HeaderName::from_static("x-authlib-injector-api-location"),
        ALI_LOCATION,
    )]
}

/// 组装 API 路由
pub fn build_app(state: AppState) -> Router {
    let texture_auth = middleware::from_fn_with_state(state.clone(), profiles::require_bearer);
    Router::new()
        // 根路径:ALI 指向 /service
        .route("/", get(root))
        // 扩展 API:元数据 + ALI
        .route("/service", get(meta::meta))
        // 认证 API
        .route("/service/authserver/authenticate", post(auth::authenticate))
        .route("/service/authserver/refresh", post(auth::refresh))
        .route("/service/authserver/validate", post(auth::validate))
        .route("/service/authserver/invalidate", post(auth::invalidate))
        .route("/service/authserver/signout", post(auth::signout))
        // 会话 API
        .route(
            "/service/sessionserver/session/minecraft/join",
            post(session::join),
        )
        .route(
            "/service/sessionserver/session/minecraft/hasJoined",
            get(session::has_joined),
        )
        .route(
            "/service/sessionserver/session/minecraft/profile/{uuid}",
            get(session::profile),
        )
        // 角色 API
        .route(
            "/service/api/profiles/minecraft",
            post(profiles::batch_profiles),
        )
        // 材质上传/清除(需 Bearer 认证)
        .route(
            "/service/api/user/profile/{uuid}/skin",
            put(profiles::upload_skin)
                .delete(profiles::delete_skin)
                .layer(texture_auth.clone()),
        )
        .route(
            "/service/api/user/profile/{uuid}/cape",
            put(profiles::upload_cape)
                .delete(profiles::delete_cape)
                .layer(texture_auth),
        )
        // 材质文件
        .route("/service/textures/{hash}", get(profiles::texture_file))
        // 旧式皮肤 API(legacy_skin_api=true 时生效)
        .route(
            "/service/skins/MinecraftSkins/{username}",
            get(profiles::legacy_skin),
        )
        // Minecraft 1.19+ 消息签名密钥
        .route(
            "/service/minecraftservices/player/certificates",
            post(certificates::certificates),
        )
        .with_state(state)
}
