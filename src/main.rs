//! yggr 服务入口:加载配置、初始化数据库/密钥/用户、启动 HTTP 服务

use anyhow::Result;
use rsa::{RsaPrivateKey, RsaPublicKey};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use yggr::api::build_app;
use yggr::app::state::{AppState, RateLimiter};
use yggr::app::textures::{DefaultSkins, TextureStore};
use yggr::app::user;
use yggr::core::config::Config;
use yggr::core::crypto;
use yggr::core::db;

#[cfg(unix)]
async fn wait_shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    tokio::select! {
        _ = tokio::signal::ctrl_c() => info!("received Ctrl+C, shutting down"),
        _ = signal(SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv() => info!("received SIGTERM, shutting down"),
    }
}

#[cfg(not(unix))]
async fn wait_shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("install Ctrl+C handler");
    info!("received Ctrl+C, shutting down");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("yggr=info,tower_http=info")),
        )
        .init();

    // 配置目录:环境变量 YGGR_CONFIG_DIR 覆盖,默认 config
    let config_dir = std::env::var("YGGR_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config"));
    let config_path = config_dir.join("config.toml");
    let mut config = if Path::new(&config_path).exists() {
        Config::load(&config_path)?
    } else {
        warn!(
            "config file {} not found, using defaults",
            config_path.display()
        );
        Config::default()
    };
    config.resolve_paths(&config_dir);
    let config = Arc::new(config);

    // 数据库
    let db_path = config.data.dir.join("yggr.db");
    let pool: SqlitePool = db::init_db(&db_path).await?;

    // RSA 密钥(自动生成)
    let key_path = config.data.dir.join("private_key.pem");
    let (private_key, public_key): (RsaPrivateKey, RsaPublicKey) =
        crypto::load_or_generate_key(&key_path)?;
    info!("signing key ready: {}", key_path.display());

    // 材质存储
    let store = TextureStore::new(&config.data.dir)?;

    // 内置默认皮肤(导入到材质存储)
    let default_skins = DefaultSkins::init(&store)?;
    info!("default skins ready");

    // 用户初始化
    user::apply_users(&config, &pool).await?;

    // 启动时清理过期令牌
    match db::delete_expired_tokens(&pool, crypto::now_millis()).await {
        Ok(n) if n > 0 => info!("cleaned {} expired tokens", n),
        Ok(_) => {}
        Err(e) => warn!("failed to clean expired tokens: {e}"),
    }

    // 应用状态
    let state = AppState {
        config: config.clone(),
        pool,
        store,
        default_skins,
        private_key: Arc::new(private_key),
        public_key,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        limiter: Arc::new(RateLimiter::new(config.auth.login_rate_limit_per_minute)),
    };

    // 定期清理过期的 join 会话与令牌
    let shutdown = CancellationToken::new();
    let cleanup_handle = {
        let state = state.clone();
        let token = shutdown.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        state.cleanup_sessions();
                        if let Err(e) = db::delete_expired_tokens(
                            &state.pool,
                            crypto::now_millis(),
                        )
                        .await
                        {
                            warn!("failed to clean expired tokens: {e}");
                        }
                    }
                    _ = token.cancelled() => {
                        info!("cleanup task stopped");
                        break;
                    }
                }
            }
        })
    };

    // HTTP 服务
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(&config.server.listen).await?;
    info!(
        "yggr {} listening on {} (meta: {}/)",
        config.meta.implementation_version, config.server.listen, config.server.base_url
    );
    let token = shutdown.clone();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        wait_shutdown_signal().await;
        token.cancel();
    })
    .await?;

    info!("server stopped, waiting for background task...");
    if let Err(e) = cleanup_handle.await {
        warn!("cleanup task join error: {e}");
    }
    info!("shutdown complete");
    Ok(())
}
