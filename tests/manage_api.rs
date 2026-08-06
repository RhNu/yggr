//! 管理 API 集成测试:角色 CRUD、皮肤模型切换、权限校验

mod common;

use axum::http::{Method, StatusCode};
use common::{TestEnv, USERNAME, authenticate, call, setup};
use serde_json::json;

#[tokio::test]
async fn manage_player_crud() {
    let env: TestEnv = setup().await;
    let (token, _) = authenticate(&env.app).await;

    // GET /api/me - 应包含预置角色 Steve
    let (status, res) = call(&env.app, Method::GET, "/api/me", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["username"], USERNAME);
    assert_eq!(res["players"].as_array().unwrap().len(), 1);
    assert_eq!(res["players"][0]["name"], "Steve");

    // POST /api/players - 创建新角色
    let (status, res) = call(
        &env.app,
        Method::POST,
        "/api/players",
        Some(json!({ "name": "Alex", "skin_model": "slim" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create player: {res}");
    assert_eq!(res["name"], "Alex");
    assert_eq!(res["skin_model"], "slim");
    let alex_id = res["id"].as_str().unwrap().to_string();

    // 再次 GET /api/me - 应有两个角色
    let (_, res) = call(&env.app, Method::GET, "/api/me", None, Some(&token)).await;
    assert_eq!(res["players"].as_array().unwrap().len(), 2);

    // PUT /api/players/{id}/skin-model - 切换为 classic
    let (status, _) = call(
        &env.app,
        Method::PUT,
        &format!("/api/players/{}/skin-model", alex_id),
        Some(json!({ "model": "classic" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 验证模型已更新
    let (_, res) = call(&env.app, Method::GET, "/api/me", None, Some(&token)).await;
    let alex = res["players"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Alex")
        .unwrap();
    assert_eq!(alex["skin_model"], "classic");

    // DELETE /api/players/{id} - 删除角色
    let (status, _) = call(
        &env.app,
        Method::DELETE,
        &format!("/api/players/{}", alex_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 验证已删除
    let (_, res) = call(&env.app, Method::GET, "/api/me", None, Some(&token)).await;
    assert_eq!(res["players"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn manage_player_name_validation() {
    let env: TestEnv = setup().await;
    let (token, _) = authenticate(&env.app).await;

    // 太短
    let (status, _) = call(
        &env.app,
        Method::POST,
        "/api/players",
        Some(json!({ "name": "ab" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 非法字符
    let (status, _) = call(
        &env.app,
        Method::POST,
        "/api/players",
        Some(json!({ "name": "bad-name!" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 重复名称
    let (status, _) = call(
        &env.app,
        Method::POST,
        "/api/players",
        Some(json!({ "name": "Steve" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn manage_player_limit() {
    let env: TestEnv = setup().await;
    let (token, _) = authenticate(&env.app).await;

    // 默认上限 5,已有 1 个(Steve),再创建 4 个达到上限
    for i in 0..4 {
        let (status, _) = call(
            &env.app,
            Method::POST,
            "/api/players",
            Some(json!({ "name": format!("Player{}", i) })),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    // 第 6 个应被拒绝
    let (status, _) = call(
        &env.app,
        Method::POST,
        "/api/players",
        Some(json!({ "name": "PlayerX" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn manage_unauthorized() {
    let env: TestEnv = setup().await;

    // 无 token
    let (status, _) = call(&env.app, Method::GET, "/api/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 伪造 token
    let (status, _) = call(
        &env.app,
        Method::GET,
        "/api/me",
        None,
        Some("fake-token-12345"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn manage_delete_nonexistent() {
    let env: TestEnv = setup().await;
    let (token, _) = authenticate(&env.app).await;

    let (status, _) = call(
        &env.app,
        Method::DELETE,
        "/api/players/00000000000000000000000000000000",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
