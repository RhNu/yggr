//! /authserver/* 认证 API:
//! - POST /service/authserver/authenticate 登录
//! - POST /service/authserver/refresh 刷新令牌
//! - POST /service/authserver/validate 验证令牌
//! - POST /service/authserver/invalidate 吊销令牌
//! - POST /service/authserver/signout 登出

use axum::extract::{ConnectInfo, Json, State};
use axum::http::{HeaderMap, StatusCode, header};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::net::{IpAddr, SocketAddr};

use crate::app::state::AppState;
use crate::core::crypto::{now_millis, random_token, random_uuid, verify_password};
use crate::core::db::{
    Token, create_token, delete_token, delete_tokens_by_user, enforce_token_limit,
    get_player_by_id, get_players_by_user, get_token, get_user_by_id, get_user_by_username,
};
use crate::core::error::{ApiError, ApiResult};
use crate::core::types::{JsonResponse, ProfileResponse, UserResponse};

/// 每个用户同时有效的令牌数上限(规范建议,如 10);超限吊销最旧的
const MAX_TOKENS_PER_USER: usize = 10;

fn db_err(e: anyhow::Error) -> ApiError {
    ApiError::internal(e.to_string())
}

/// 令牌状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStatus {
    /// 有效:可进行所有操作
    Valid,
    /// 暂时失效:仅可刷新(token_active_window 超时但未过期)
    TemporarilyInvalid,
    /// 无效:已过期
    Invalid,
}

/// 判定令牌状态(根据配置的 active window)
pub fn token_status(tok: &Token, config: &crate::core::config::Config) -> TokenStatus {
    let now = now_millis();
    if tok.expires_at <= now {
        return TokenStatus::Invalid;
    }
    if config.temp_invalidation_enabled() {
        let active_ms = config.auth.token_active_window_days * 24 * 3600 * 1000;
        if tok.issued_at + active_ms <= now {
            return TokenStatus::TemporarilyInvalid;
        }
    }
    TokenStatus::Valid
}

/// 令牌是否完全有效(非暂时失效、非过期)
fn token_valid(tok: &Token, config: &crate::core::config::Config) -> bool {
    token_status(tok, config) == TokenStatus::Valid
}

/// 从请求提取客户端 IP(优先 X-Forwarded-For,用于反代场景)
pub fn client_ip(headers: &HeaderMap, addr: SocketAddr) -> IpAddr {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first) = xff.split(',').next()
        && let Ok(ip) = first.trim().parse()
    {
        return ip;
    }
    addr.ip()
}

// ---- 请求/响应结构 ----

#[derive(Debug, Deserialize)]
pub struct Agent {
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    version: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateRequest {
    pub username: String,
    pub password: String,
    pub client_token: Option<String>,
    pub request_user: Option<bool>,
    #[allow(dead_code)]
    pub agent: Option<Agent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateResponse {
    pub access_token: String,
    pub client_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_profiles: Option<Vec<ProfileResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<ProfileResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub access_token: String,
    pub client_token: Option<String>,
    pub request_user: Option<bool>,
    pub selected_profile: Option<ProfileSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSelection {
    pub id: String,
    #[allow(dead_code)]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub client_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<ProfileResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateRequest {
    pub access_token: String,
    pub client_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidateRequest {
    pub access_token: String,
    #[allow(dead_code)]
    pub client_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SignoutRequest {
    pub username: String,
    pub password: String,
}

// ---- Handlers ----

/// POST /service/authserver/authenticate
pub async fn authenticate(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<AuthenticateRequest>,
) -> ApiResult<JsonResponse<AuthenticateResponse>> {
    // 限流:按客户端 IP(防全网爆破)+ 按用户名(规范建议针对用户,防单人爆破)
    let ip = client_ip(&headers, addr);
    if !state.limiter.check(&ip.to_string()) || !state.limiter.check(&req.username) {
        // 规范:短时间内多次登录失败按密码错误处理
        return Err(ApiError::invalid_credentials());
    }

    let pool: &SqlitePool = &state.pool;

    // 用户查找:先按用户名(邮箱),non_email_login 时再按角色名
    let mut login_player = None;
    let user = if let Some(u) = get_user_by_username(pool, &req.username)
        .await
        .map_err(db_err)?
    {
        u
    } else if state.config.auth.non_email_login {
        match crate::core::db::get_player_by_name(pool, &req.username)
            .await
            .map_err(db_err)?
        {
            Some(p) => {
                login_player = Some(p.clone());
                get_user_by_id(pool, &p.user_id)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(ApiError::invalid_credentials)?
            }
            None => return Err(ApiError::invalid_credentials()),
        }
    } else {
        return Err(ApiError::invalid_credentials());
    };

    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::invalid_credentials());
    }

    // 角色绑定规则:角色名登录绑定该角色;仅一个角色自动绑定;多角色由客户端选择
    let players = get_players_by_user(pool, &user.id).await.map_err(db_err)?;
    let selected = if let Some(p) = &login_player {
        Some(p.clone())
    } else if players.len() == 1 {
        Some(players[0].clone())
    } else {
        None
    };

    // 颁发令牌
    let client_token = req.client_token.unwrap_or_else(random_uuid);
    let access_token = random_token();
    let now = now_millis();
    let ttl = state.config.auth.token_ttl_days * 24 * 3600 * 1000;
    create_token(
        pool,
        &access_token,
        &client_token,
        &user.id,
        selected.as_ref().map(|p| p.id.as_str()),
        now,
        now + ttl,
    )
    .await
    .map_err(db_err)?;
    // 令牌数量上限:超限吊销该用户最旧的令牌
    enforce_token_limit(pool, &user.id, MAX_TOKENS_PER_USER)
        .await
        .map_err(db_err)?;

    let available_profiles = players.iter().map(ProfileResponse::basic).collect();
    let user_response = if req.request_user.unwrap_or(false) {
        Some(UserResponse::from_user(&user))
    } else {
        None
    };

    Ok(JsonResponse(AuthenticateResponse {
        access_token,
        client_token,
        available_profiles: Some(available_profiles),
        selected_profile: selected.as_ref().map(ProfileResponse::basic),
        user: user_response,
    }))
}

/// POST /service/authserver/refresh
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> ApiResult<JsonResponse<RefreshResponse>> {
    let pool = &state.pool;

    let tok = get_token(pool, &req.access_token)
        .await
        .map_err(db_err)?
        .ok_or_else(ApiError::invalid_token)?;
    // refresh 接受有效与暂时失效的令牌
    let status = token_status(&tok, &state.config);
    if status == TokenStatus::Invalid {
        return Err(ApiError::invalid_token());
    }
    if let Some(ct) = &req.client_token
        && &tok.client_token != ct
    {
        return Err(ApiError::invalid_token());
    }

    // 角色选择
    let players = get_players_by_user(pool, &tok.user_id)
        .await
        .map_err(db_err)?;
    let mut new_player_id = tok.player_id.clone();
    if let Some(sel) = &req.selected_profile {
        if tok.player_id.is_some() {
            return Err(ApiError::profile_already_assigned());
        }
        if !players.iter().any(|p| p.id == sel.id) {
            return Err(ApiError::invalid_profile_selection());
        }
        new_player_id = Some(sel.id.clone());
    }

    // 颁发新令牌(相同 clientToken);先建后删,保证失败时原令牌依然有效
    let access_token = random_token();
    let now = now_millis();
    let ttl = state.config.auth.token_ttl_days * 24 * 3600 * 1000;
    create_token(
        pool,
        &access_token,
        &tok.client_token,
        &tok.user_id,
        new_player_id.as_deref(),
        now,
        now + ttl,
    )
    .await
    .map_err(db_err)?;
    // 新令牌已成功创建,现在吊销原令牌
    delete_token(pool, &tok.access_token)
        .await
        .map_err(db_err)?;
    // 令牌数量上限兜底(数量不变,防御性调用)
    enforce_token_limit(pool, &tok.user_id, MAX_TOKENS_PER_USER)
        .await
        .map_err(db_err)?;

    // 响应
    let selected_profile = match &new_player_id {
        Some(pid) => get_player_by_id(pool, pid)
            .await
            .map_err(db_err)?
            .map(|p| ProfileResponse::basic(&p)),
        None => None,
    };
    let user_response = if req.request_user.unwrap_or(false) {
        get_user_by_id(pool, &tok.user_id)
            .await
            .map_err(db_err)?
            .map(|u| UserResponse::from_user(&u))
    } else {
        None
    };

    Ok(JsonResponse(RefreshResponse {
        access_token,
        client_token: tok.client_token,
        selected_profile,
        user: user_response,
    }))
}

/// POST /service/authserver/validate;有效返回 204
pub async fn validate(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> ApiResult<StatusCode> {
    let tok = get_token(&state.pool, &req.access_token)
        .await
        .map_err(db_err)?
        .ok_or_else(ApiError::invalid_token)?;
    if !token_valid(&tok, &state.config) {
        return Err(ApiError::invalid_token());
    }
    if let Some(ct) = &req.client_token
        && &tok.client_token != ct
    {
        return Err(ApiError::invalid_token());
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /service/authserver/invalidate;无论结果如何返回 204
pub async fn invalidate(
    State(state): State<AppState>,
    Json(req): Json<InvalidateRequest>,
) -> ApiResult<StatusCode> {
    if let Some(tok) = get_token(&state.pool, &req.access_token)
        .await
        .map_err(db_err)?
    {
        delete_token(&state.pool, &tok.access_token)
            .await
            .map_err(db_err)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /service/authserver/signout;吊销用户所有令牌
pub async fn signout(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<SignoutRequest>,
) -> ApiResult<StatusCode> {
    // 限流:IP + 用户名双维度(与 authenticate 同等强度)
    let ip = client_ip(&headers, addr);
    if !state.limiter.check(&ip.to_string()) || !state.limiter.check(&req.username) {
        return Err(ApiError::invalid_credentials());
    }
    let user = get_user_by_username(&state.pool, &req.username)
        .await
        .map_err(db_err)?
        .ok_or_else(ApiError::invalid_credentials)?;
    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::invalid_credentials());
    }
    delete_tokens_by_user(&state.pool, &user.id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- 辅助:Bearer 认证(材质上传、certificates) ----

/// 从 Authorization: Bearer 头解析并校验令牌;无效返回 401
pub async fn bearer_token(state: &AppState, headers: &HeaderMap) -> Result<Token, ApiError> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(ApiError::unauthorized)?;
    let token = auth
        .strip_prefix("Bearer ")
        .ok_or_else(ApiError::unauthorized)?;
    let tok = get_token(&state.pool, token)
        .await
        .map_err(db_err)?
        .ok_or_else(ApiError::unauthorized)?;
    if !token_valid(&tok, &state.config) {
        return Err(ApiError::unauthorized());
    }
    Ok(tok)
}
