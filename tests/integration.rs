//! 端到端集成测试:走通 Yggdrasil 全流程
//! (meta → authenticate → validate → join → hasJoined → profile → 材质上传 → refresh → invalidate → signout)

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

use yggr::build_app;
use yggr::config::Config;
use yggr::crypto;
use yggr::db;
use yggr::state::{AppState, RateLimiter};
use yggr::textures::TextureStore;

const TEST_PASSWORD: &str = "test-password-123";
const PLAYER_NAME: &str = "Steve";
const USERNAME: &str = "admin@example.com";

struct TestEnv {
    app: Router,
    player_id: String,
    /// 128x64 的测试皮肤 PNG(RGBA)
    skin_png: Vec<u8>,
    public_key: rsa::RsaPublicKey,
}

async fn setup() -> TestEnv {
    let dir = std::env::temp_dir().join(format!("yggr-test-{}", uuid::Uuid::new_v4()));
    let config = Config {
        base_url: "http://127.0.0.1:8080".to_string(),
        data_dir: dir.clone(),
        seed_file: None,
        login_rate_limit_per_minute: 1000,
        ..Config::default()
    };
    let config = Arc::new(config);
    let pool = db::init_db(&dir.join("test.db")).await.unwrap();
    let (private_key, public_key) = crypto::generate_keypair().unwrap();
    let store = TextureStore::new(&dir).unwrap();

    // 创建用户与角色
    let user_id = crypto::random_uuid();
    let password_hash = crypto::hash_password(TEST_PASSWORD).unwrap();
    db::create_user(&pool, &user_id, USERNAME, &password_hash, "zh_CN")
        .await
        .unwrap();
    let player_id = crypto::offline_uuid(PLAYER_NAME);
    db::create_player(&pool, &player_id, PLAYER_NAME, &user_id, None, None, "classic")
        .await
        .unwrap();

    let state = AppState {
        config,
        pool,
        store,
        private_key: Arc::new(private_key),
        public_key: public_key.clone(),
        sessions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        limiter: Arc::new(RateLimiter::new(1000)),
    };
    let app = build_app(state);

    // 生成 128x64 RGBA 测试皮肤
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

    TestEnv {
        app,
        player_id,
        skin_png: out,
        public_key,
    }
}

async fn call(
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

async fn authenticate(app: &Router) -> (String, String) {
    let (status, res) = call(
        app,
        Method::POST,
        "/authserver/authenticate",
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

#[tokio::test]
async fn full_yggdrasil_flow() {
    let env = setup().await;
    let app = &env.app;

    // 1. meta
    let (status, meta) = call(app, Method::GET, "/", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(meta["signaturePublickey"]
        .as_str()
        .unwrap()
        .starts_with("-----BEGIN PUBLIC KEY-----"));
    assert_eq!(meta["meta"]["implementationName"], "yggr");
    assert!(meta["skinDomains"].is_array());

    // 2. authenticate
    let (access_token, client_token) = authenticate(app).await;

    // 3. validate
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/validate",
        Some(json!({"accessToken": access_token, "clientToken": client_token})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 4. 错误密码 → 403 ForbiddenOperationException
    let (status, err) = call(
        app,
        Method::POST,
        "/authserver/authenticate",
        Some(json!({"username": USERNAME, "password": "wrong"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(err["error"], "ForbiddenOperationException");

    // 5. join → hasJoined
    let server_id = "server-test-123";
    let (status, _) = call(
        app,
        Method::POST,
        "/sessionserver/session/minecraft/join",
        Some(json!({
            "accessToken": access_token,
            "selectedProfile": env.player_id,
            "serverId": server_id
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, profile) = call(
        app,
        Method::GET,
        &format!(
            "/sessionserver/session/minecraft/hasJoined?username={}&serverId={}",
            PLAYER_NAME, server_id
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["name"], PLAYER_NAME);
    assert_eq!(profile["id"], env.player_id);

    // 已使用过的 serverId 不能重放
    let (status, _) = call(
        app,
        Method::GET,
        &format!(
            "/sessionserver/session/minecraft/hasJoined?username={}&serverId={}",
            PLAYER_NAME, server_id
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 错误的角色 join → 403
    let (status, _) = call(
        app,
        Method::POST,
        "/sessionserver/session/minecraft/join",
        Some(json!({
            "accessToken": access_token,
            "selectedProfile": "00000000000000000000000000000000",
            "serverId": "server-bad"
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 6. profile 查询
    let (status, profile) = call(
        app,
        Method::GET,
        &format!("/sessionserver/session/minecraft/profile/{}", env.player_id),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["id"], env.player_id);
    // 无材质时 properties 为空数组
    assert_eq!(profile["properties"], json!([]));

    // 7. 批量查询
    let (status, res) = call(
        app,
        Method::POST,
        "/api/profiles/minecraft",
        Some(json!([PLAYER_NAME, "Nobody"])),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res.as_array().unwrap().len(), 1);
    assert_eq!(res[0]["name"], PLAYER_NAME);

    // 8. 上传皮肤 → profile 含签名 textures → 下载材质
    let boundary = "----yggr-test-boundary";
    let multipart = format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nslim\r\n--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\nContent-Type: image/png\r\n\r\n",
        b = boundary
    )
    .into_bytes()
    .into_iter()
    .chain(env.skin_png.clone())
    .chain(format!("\r\n--{}--\r\n", boundary).into_bytes())
    .collect::<Vec<_>>();

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/user/profile/{}/skin", env.player_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .extension(axum::extract::ConnectInfo(SocketAddr::from((
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            12345,
        ))))
        .body(Body::from(multipart))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 未认证上传 → 401
    let (status, _) = call(
        app,
        Method::PUT,
        &format!("/api/user/profile/{}/skin", env.player_id),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // profile 含签名 textures 属性
    let (status, profile) = call(
        app,
        Method::GET,
        &format!(
            "/sessionserver/session/minecraft/profile/{}?unsigned=false",
            env.player_id
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let textures_prop = profile["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "textures")
        .expect("textures property");
    let signature = textures_prop["signature"].as_str().unwrap();
    assert!(!signature.is_empty());
    let value_b64 = textures_prop["value"].as_str().unwrap();
    // 验签
    let sig = base64::engine::general_purpose::STANDARD
        .decode(signature)
        .unwrap();
    assert!(crypto::verify_sha1(&env.public_key, value_b64.as_bytes(), &sig));

    // 材质文件可下载
    let decoded: Value = serde_json::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(value_b64)
            .unwrap(),
    )
    .unwrap();
    let url = decoded["textures"]["SKIN"]["url"].as_str().unwrap();
    let hash = url.rsplit('/').next().unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/textures/{}", hash))
        .extension(axum::extract::ConnectInfo(SocketAddr::from((
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            12345,
        ))))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()[header::CONTENT_TYPE],
        "image/png"
    );

    // 9. refresh
    let (status, refreshed) = call(
        app,
        Method::POST,
        "/authserver/refresh",
        Some(json!({
            "accessToken": access_token,
            "clientToken": client_token
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(refreshed["clientToken"], client_token);
    let new_token = refreshed["accessToken"].as_str().unwrap().to_string();
    assert_ne!(new_token, access_token);

    // 旧令牌已吊销
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/validate",
        Some(json!({"accessToken": access_token})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 10. certificates
    let (status, certs) = call(
        app,
        Method::POST,
        "/minecraftservices/player/certificates",
        None,
        Some(&new_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!certs["keyPair"]["privateKey"]
        .as_str()
        .unwrap()
        .is_empty());
    assert!(!certs["keyPair"]["publicKeySignatureV2"]
        .as_str()
        .unwrap()
        .is_empty());
    assert!(certs["expiresAt"].as_str().unwrap().starts_with("20"));

    // 11. invalidate
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/invalidate",
        Some(json!({"accessToken": new_token})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/validate",
        Some(json!({"accessToken": new_token})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 12. signout 吊销全部令牌
    let (_, token2) = authenticate(app).await;
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/signout",
        Some(json!({"username": USERNAME, "password": TEST_PASSWORD})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/validate",
        Some(json!({"accessToken": token2})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn non_email_login_and_role_name() {
    let env = setup().await;
    let app = &env.app;

    // 使用角色名登录(用户存在时用户名优先,这里用户名为邮箱,角色名可登录)
    let (status, res) = call(
        app,
        Method::POST,
        "/authserver/authenticate",
        Some(json!({
            "username": PLAYER_NAME,
            "password": TEST_PASSWORD
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // 角色名登录自动绑定角色
    assert_eq!(res["selectedProfile"]["name"], PLAYER_NAME);
}

#[tokio::test]
async fn refresh_with_profile_selection_errors() {
    let env = setup().await;
    let app = &env.app;
    let (access_token, _) = authenticate(app).await;

    // 令牌已绑定角色,再指定角色 → 400
    let (status, err) = call(
        app,
        Method::POST,
        "/authserver/refresh",
        Some(json!({
            "accessToken": access_token,
            "selectedProfile": {"id": env.player_id, "name": PLAYER_NAME}
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "IllegalArgumentException");

    // 无效令牌 → 403
    let (status, _) = call(
        app,
        Method::POST,
        "/authserver/validate",
        Some(json!({"accessToken": "invalid-token"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
