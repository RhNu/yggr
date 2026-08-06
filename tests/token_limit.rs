//! 令牌上限测试:超限吊销最旧令牌

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn token_limit_revokes_oldest() {
    let env = setup().await;
    let app = &env.app;

    // 连续登录 11 次(上限 auth.rs::MAX_TOKENS_PER_USER = 10)
    let mut tokens = Vec::new();
    for _ in 0..11 {
        let (status, res) = call(
            app,
            Method::POST,
            "/service/authserver/authenticate",
            Some(json!({"username": USERNAME, "password": TEST_PASSWORD})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        tokens.push(res["accessToken"].as_str().unwrap().to_string());
        // 保证 issued_at(毫秒)有区分度,使"吊销最旧"断言确定(避免同毫秒退化为随机排序)
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    // 最早颁发的令牌已被吊销(超限吊销最旧)
    let (status, _) = call(
        app,
        Method::POST,
        "/service/authserver/validate",
        Some(json!({"accessToken": tokens[0]})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 最新令牌仍然有效
    let (status, _) = call(
        app,
        Method::POST,
        "/service/authserver/validate",
        Some(json!({"accessToken": tokens[10]})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
