//! 认证边缘场景:角色名登录、refresh 角色选择错误、无效令牌

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

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

    // 令牌已绑定角色,再指定角色 -> 400
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

    // 无效令牌 -> 403
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
