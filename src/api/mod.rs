//! HTTP API 层:authlib-injector 规范端点 + 管理 API + 前端静态服务
//!
//! 本层依赖 `core`(基础设施)与 `app`(服务层),负责路由注册与请求处理。
//! 所有 Yggdrasil 规范端点挂载在 `/service` 下;
//! 管理 API 挂载在 `/api` 下;根路径 `/` 提供前端入口与 ALI 头。

mod auth;
mod certificates;
mod manage;
mod meta;
mod profiles;
mod session;

use axum::Router;
use axum::body::Body;
use axum::http::{Response, header};
use axum::middleware;
use axum::routing::{get, post, put};
use tower_http::services::ServeDir;

use crate::app::state::AppState;

/// ALI 头值:指向 /service
const ALI_LOCATION: &str = "/service";

/// 根路径 GET / - 返回 ALI 头;若前端 index.html 存在则同时返回前端入口
async fn root(axum::extract::State(state): axum::extract::State<AppState>) -> Response<Body> {
    let builder = Response::builder().header("x-authlib-injector-api-location", ALI_LOCATION);
    if let Some(dir) = &state.config.frontend.dir {
        let index = dir.join("index.html");
        if let Ok(data) = std::fs::read(&index) {
            return builder
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(data))
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }
    }
    builder
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

/// 组装 API 路由
pub fn build_app(state: AppState) -> Router {
    let texture_auth = middleware::from_fn_with_state(state.clone(), profiles::require_bearer);
    let manage_auth = middleware::from_fn_with_state(state.clone(), profiles::require_bearer);

    let spec_routes = Router::new()
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
        // 默认皮肤(classic|slim)
        .route(
            "/service/textures/default/{model}",
            get(profiles::default_skin),
        )
        // 旧式皮肤 API(legacy_skin_api=true 时生效)
        .route(
            "/service/skins/MinecraftSkins/{username}",
            get(profiles::legacy_skin),
        )
        // Minecraft 1.19+ 消息签名密钥
        .route(
            "/service/minecraftservices/player/certificates",
            post(certificates::certificates),
        );

    // 管理 API(需 Bearer 认证)
    let manage_routes = Router::new()
        .route("/api/me", get(manage::me))
        .route("/api/players", post(manage::create_player_handler))
        .route(
            "/api/players/{id}",
            axum::routing::delete(manage::delete_player_handler),
        )
        .route(
            "/api/players/{id}/skin-model",
            put(manage::update_skin_model_handler),
        )
        .layer(manage_auth);

    let mut router = Router::new()
        // 根路径:ALI 头 + 前端入口
        .route("/", get(root))
        .merge(spec_routes)
        .merge(manage_routes)
        .with_state(state.clone());

    // 前端静态文件 fallback
    if let Some(dir) = &state.config.frontend.dir
        && dir.exists()
    {
        router = router.fallback_service(ServeDir::new(dir));
    }

    router
}
