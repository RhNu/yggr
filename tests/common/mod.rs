//! 集成测试共享基础设施:测试环境、HTTP 调用、认证辅助
#![allow(dead_code)]

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

use yggr::api::build_app;
use yggr::app::state::{AppState, RateLimiter};
use yggr::app::textures::{DefaultSkins, TextureStore};
use yggr::core::config::Config;
use yggr::core::crypto;
use yggr::core::db;

pub const TEST_PASSWORD: &str = "test-password-123";
pub const PLAYER_NAME: &str = "Steve";
pub const USERNAME: &str = "admin@example.com";

pub struct TestEnv {
    pub app: Router,
    pub player_id: String,
    /// 128x64 的测试皮肤 PNG(RGBA)
    pub skin_png: Vec<u8>,
    pub public_key: rsa::RsaPublicKey,
}

/// 生成 128x64 RGBA 测试皮肤 PNG
fn make_skin_png() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 128, 64);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&vec![0x80u8; 128 * 64 * 4])
            .unwrap();
    }
    out
}

pub async fn setup() -> TestEnv {
    let dir = std::env::temp_dir().join(format!("yggr-test-{}", uuid::Uuid::new_v4()));
    let config = Config {
        server: yggr::core::config::ServerConfig {
            base_url: "http://127.0.0.1:8080".to_string(),
            ..Default::default()
        },
        data: yggr::core::config::DataConfig { dir: dir.clone() },
        auth: yggr::core::config::AuthConfig {
            login_rate_limit_per_minute: 1000,
            ..Default::default()
        },
        seed: yggr::core::config::SeedConfig { file: None },
        ..Config::default()
    };
    let config = Arc::new(config);
    let pool = db::init_db(&dir.join("test.db")).await.unwrap();
    let (private_key, public_key) = crypto::generate_keypair_with_size(2048).unwrap();
    let store = TextureStore::new(&dir).unwrap();
    let default_skins = DefaultSkins::init(&store).unwrap();

    // 创建用户与角色
    let user_id = crypto::random_uuid();
    let password_hash = crypto::hash_password(TEST_PASSWORD).unwrap();
    db::create_user(&pool, &user_id, USERNAME, &password_hash, "zh_CN")
        .await
        .unwrap();
    let player_id = crypto::offline_uuid(PLAYER_NAME);
    db::create_player(
        &pool,
        &player_id,
        PLAYER_NAME,
        &user_id,
        None,
        None,
        "classic",
    )
    .await
    .unwrap();

    let state = AppState {
        config,
        pool,
        store,
        default_skins,
        private_key: Arc::new(private_key),
        public_key: public_key.clone(),
        sessions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        limiter: Arc::new(RateLimiter::new(1000)),
    };
    let app = build_app(state);

    TestEnv {
        app,
        player_id,
        skin_png: make_skin_png(),
        public_key,
    }
}

pub async fn call(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(b) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", b));
    }
    if let Some(json) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        let req = builder
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                12345,
            ))))
            .body(Body::from(json.to_string()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, value)
    } else {
        let req = builder
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                12345,
            ))))
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, value)
    }
}

pub async fn authenticate(app: &Router) -> (String, String) {
    let (status, res) = call(
        app,
        Method::POST,
        "/service/authserver/authenticate",
        Some(json!({
            "username": USERNAME,
            "password": TEST_PASSWORD,
            "requestUser": true
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["selectedProfile"]["name"], PLAYER_NAME);
    assert_eq!(res["availableProfiles"][0]["name"], PLAYER_NAME);
    assert_eq!(res["user"]["properties"][0]["name"], "preferredLanguage");
    (
        res["accessToken"].as_str().unwrap().to_string(),
        res["clientToken"].as_str().unwrap().to_string(),
    )
}
