//! /sessionserver/* 会话 API:
//! - POST /service/sessionserver/session/minecraft/join 客户端进入服务器
//! - GET /service/sessionserver/session/minecraft/hasJoined 服务端验证客户端
//! - GET /service/sessionserver/session/minecraft/profile/{uuid} 查询角色属性

use axum::Json;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use base64::Engine;
use serde::Deserialize;
use std::net::SocketAddr;

use crate::api::auth::client_ip;
use crate::app::state::{AppState, JoinRecord};
use crate::app::textures::build_textures_value;
use crate::core::crypto::{now_millis, sign_sha1};
use crate::core::db::{Player, get_player_by_id, get_token};
use crate::core::error::{ApiError, ApiResult};
use crate::core::types::{JsonResponse, ProfileResponse, Property};
use tracing::{debug, info, instrument, warn};

/// join 会话有效期(毫秒)
const JOIN_TTL_MS: i64 = 30_000;

fn db_err(e: anyhow::Error) -> ApiError {
    tracing::error!(error = %e, "database error in session");
    ApiError::internal(e.to_string())
}

/// 构建角色完整响应(含签名 textures 属性与 uploadableTextures)
pub fn build_profile_response(
    state: &AppState,
    player: &Player,
    signed: bool,
) -> ApiResult<ProfileResponse> {
    let mut properties = Vec::new();
    // 始终输出 textures 属性(无皮肤时回退到内置默认皮肤)
    let value = build_textures_value(&state.config, &state.default_skins, player)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if signed {
        let sig = sign_sha1(&state.private_key, value.as_bytes())
            .map_err(|e| ApiError::internal(e.to_string()))?;
        properties.push(Property::signed(
            "textures",
            &value,
            base64::engine::general_purpose::STANDARD.encode(sig),
        ));
    } else {
        properties.push(Property::plain("textures", &value));
    }
    // uploadableTextures(authlib-injector 扩展属性):该角色可上传的材质类型;
    // yggr 支持皮肤与披风上传,故输出 "skin,cape"
    properties.push(Property::plain("uploadableTextures", "skin,cape"));
    Ok(ProfileResponse::full(player, properties))
}

// ---- join ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    pub access_token: String,
    pub selected_profile: String,
    pub server_id: String,
}

/// POST /service/sessionserver/session/minecraft/join
#[instrument(skip_all, level = "debug")]
pub async fn join(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<JoinRequest>,
) -> ApiResult<StatusCode> {
    let tok = get_token(&state.pool, &req.access_token)
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            debug!("join: token not found");
            ApiError::invalid_token()
        })?;
    if crate::api::auth::token_status(&tok, &state.config) != crate::api::auth::TokenStatus::Valid {
        warn!(user_id = %tok.user_id, "join: token invalid");
        return Err(ApiError::invalid_token());
    }
    if tok.player_id.as_deref() != Some(req.selected_profile.as_str()) {
        warn!(user_id = %tok.user_id, "join: profile mismatch");
        return Err(ApiError::invalid_token());
    }
    let player = get_player_by_id(&state.pool, &req.selected_profile)
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            debug!("join: player not found: {}", req.selected_profile);
            ApiError::invalid_token()
        })?;

    let ip = client_ip(&headers, addr).to_string();
    let expires_at = now_millis() + JOIN_TTL_MS;
    state.sessions.lock().unwrap().insert(
        req.server_id.clone(),
        JoinRecord {
            player_id: player.id.clone(),
            player_name: player.name.clone(),
            ip: Some(ip),
            expires_at,
        },
    );
    info!(player = %player.name, player_id = %player.id, "session joined");
    Ok(StatusCode::NO_CONTENT)
}

// ---- hasJoined ----

#[derive(Debug, Deserialize)]
pub struct HasJoinedQuery {
    pub username: String,
    #[serde(rename = "serverId")]
    pub server_id: String,
    pub ip: Option<String>,
}

/// GET /service/sessionserver/session/minecraft/hasJoined
#[instrument(skip_all, level = "debug")]
pub async fn has_joined(
    State(state): State<AppState>,
    Query(query): Query<HasJoinedQuery>,
) -> ApiResult<Result<JsonResponse<ProfileResponse>, StatusCode>> {
    let now = now_millis();
    let record: Option<JoinRecord> = {
        let mut map = state.sessions.lock().unwrap();
        let rec = map.get(&query.server_id).cloned();
        map.remove(&query.server_id);
        rec
    };
    let Some(record) = record else {
        debug!(server_id = %query.server_id, "hasJoined: no session");
        return Ok(Err(StatusCode::NO_CONTENT));
    };
    if record.expires_at <= now {
        debug!(player = %record.player_name, "hasJoined: session expired");
        return Ok(Err(StatusCode::NO_CONTENT));
    }
    if record.player_name != query.username {
        debug!(expected = %record.player_name, actual = %query.username, "hasJoined: name mismatch");
        return Ok(Err(StatusCode::NO_CONTENT));
    }
    // IP 校验(可选)
    if state.config.auth.check_ip
        && let Some(expected) = &record.ip
        && let Some(actual) = &query.ip
        && actual != expected
    {
        return Ok(Err(StatusCode::NO_CONTENT));
    }

    let player = get_player_by_id(&state.pool, &record.player_id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            debug!(player_id = %record.player_id, "hasJoined: player not found");
            ApiError::invalid_token()
        })?;
    let profile = build_profile_response(&state, &player, true)?;
    info!(player = %record.player_name, "hasJoined: verified");
    Ok(Ok(JsonResponse(profile)))
}

// ---- profile 查询 ----

#[derive(Debug, Deserialize)]
pub struct ProfileQuery {
    pub unsigned: Option<String>,
}

/// GET /service/sessionserver/session/minecraft/profile/{uuid}
#[instrument(skip_all, level = "debug")]
pub async fn profile(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    Query(query): Query<ProfileQuery>,
) -> ApiResult<Result<JsonResponse<ProfileResponse>, StatusCode>> {
    let Some(player) = get_player_by_id(&state.pool, &uuid).await.map_err(db_err)? else {
        debug!(uuid = %uuid, "profile: player not found");
        return Ok(Err(StatusCode::NO_CONTENT));
    };
    // unsigned=false 时包含签名;默认 true(不含签名)
    let signed = query.unsigned.as_deref() == Some("false");
    let profile = build_profile_response(&state, &player, signed)?;
    Ok(Ok(JsonResponse(profile)))
}
