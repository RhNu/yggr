//! yggr 服务入口:加载配置、初始化数据库/密钥/种子、启动 HTTP 服务

use anyhow::Result;
use rsa::{RsaPrivateKey, RsaPublicKey};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use yggr::api::build_app;
use yggr::app::seed;
use yggr::app::state::{AppState, RateLimiter};
use yggr::app::textures::TextureStore;
use yggr::core::config::Config;
use yggr::core::crypto;
use yggr::core::db;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("yggr=info,tower_http=info")),
        )
        .init();

    // 配置:第一个命令行参数为配置文件路径,默认 config.toml
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());
    let config = if Path::new(&config_path).exists() {
        Config::load(Path::new(&config_path))?
    } else {
        warn!("config file {} not found, using defaults", config_path);
        Config::default()
    };
    let config = Arc::new(config);

    // 数据库
    let db_path = config.data_dir.join("yggr.db");
    let pool: SqlitePool = db::init_db(&db_path).await?;

    // RSA 密钥(自动生成)
    let key_path = config.data_dir.join("private_key.pem");
    let (private_key, public_key): (RsaPrivateKey, RsaPublicKey) =
        crypto::load_or_generate_key(&key_path)?;
    info!("signing key ready: {}", key_path.display());

    // 材质存储
    let store = TextureStore::new(&config.data_dir)?;

    // 种子用户初始化
    seed::apply_seed(&config, &pool, &store).await?;

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
        private_key: Arc::new(private_key),
        public_key,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        limiter: Arc::new(RateLimiter::new(config.login_rate_limit_per_minute)),
    };

    // 定期清理过期的 join 会话与令牌
    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                state.cleanup_sessions();
                if let Err(e) = db::delete_expired_tokens(&state.pool, crypto::now_millis()).await {
                    warn!("failed to clean expired tokens: {e}");
                }
            }
        });
    }

    // HTTP 服务
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
    info!(
        "yggr {} listening on {} (meta: {}/)",
        config.implementation_version, config.listen, config.base_url
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
