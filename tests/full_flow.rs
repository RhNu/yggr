//! 端到端集成测试:走通 Yggdrasil 全流程
//! (meta -> authenticate -> validate -> join -> hasJoined -> profile -> 材质上传 -> refresh -> invalidate -> signout)

mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use base64::Engine;
use common::*;
use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::ServiceExt;
use yggr::core::crypto;

#[tokio::test]
async fn full_yggdrasil_flow() {
    let env = setup().await;
    let app = &env.app;

    // 1. meta
    let (status, meta) = call(app, Method::GET, "/", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        meta["signaturePublickey"]
            .as_str()
            .unwrap()
            .starts_with("-----BEGIN PUBLIC KEY-----")
    );
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

    // 4. 错误密码 -> 403 ForbiddenOperationException
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

    // 5. join -> hasJoined
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

    // 错误的角色 join -> 403
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
    // 无材质时 properties 仅包含 uploadableTextures 属性
    assert_eq!(
        profile["properties"],
        json!([{"name": "uploadableTextures", "value": "skin,cape"}])
    );

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

    // 8. 上传皮肤 -> profile 含签名 textures -> 下载材质
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

    // 未认证上传 -> 401
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
    assert!(crypto::verify_sha1(
        &env.public_key,
        value_b64.as_bytes(),
        &sig
    ));

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
    assert_eq!(res.headers()[header::CONTENT_TYPE], "image/png");

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
    assert!(!certs["keyPair"]["privateKey"].as_str().unwrap().is_empty());
    assert!(
        !certs["keyPair"]["publicKeySignatureV2"]
            .as_str()
            .unwrap()
            .is_empty()
    );
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
