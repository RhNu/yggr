//! 管理 API:角色 CRUD、皮肤模型切换
//!
//! 所有端点需 Bearer 认证,挂载在 `/api` 下(与 `/service` 规范端点区分)。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

use crate::api::auth::bearer_token;
use crate::app::state::AppState;
use crate::core::config::UuidGeneration;
use crate::core::crypto::{offline_uuid, random_uuid};
use crate::core::db::{
    Player, count_players_by_user, create_player, delete_player, get_player_by_id,
    get_players_by_user, update_skin_model,
};
use crate::core::error::{ApiError, ApiResult};
use crate::core::types::JsonResponse;
use tracing::{debug, info, instrument, warn};

fn db_err(e: anyhow::Error) -> ApiError {
    tracing::error!(error = %e, "database error in manage");
    ApiError::internal(e.to_string())
}

/// 角色名规则:3-16 字符,仅 a-zA-Z0-9_
fn validate_player_name(name: &str) -> Result<(), ApiError> {
    let len = name.chars().count();
    if !(3..=16).contains(&len) {
        return Err(ApiError::bad_request("Player name must be 3-16 characters"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError::bad_request(
            "Player name may only contain letters, digits and underscores",
        ));
    }
    Ok(())
}

/// 角色 DTO(前端管理用)
#[derive(Debug, Serialize)]
pub struct PlayerDto {
    pub id: String,
    pub name: String,
    pub skin_hash: Option<String>,
    pub cape_hash: Option<String>,
    pub skin_model: String,
}

impl From<Player> for PlayerDto {
    fn from(p: Player) -> Self {
        PlayerDto {
            id: p.id,
            name: p.name,
            skin_hash: p.skin_hash,
            cape_hash: p.cape_hash,
            skin_model: p.skin_model,
        }
    }
}

/// GET /api/me - 当前用户信息 + 角色列表
#[instrument(skip_all, level = "debug")]
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<JsonResponse<MeResponse>> {
    let tok = bearer_token(&state, &headers).await?;
    let user = crate::core::db::get_user_by_id(&state.pool, &tok.user_id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| ApiError::internal("user not found"))?;
    let players = get_players_by_user(&state.pool, &tok.user_id)
        .await
        .map_err(db_err)?;
    Ok(JsonResponse(MeResponse {
        username: user.username,
        preferred_language: user.preferred_language,
        players: players.into_iter().map(PlayerDto::from).collect(),
    }))
}

/// 请求体:创建角色
#[derive(Debug, Deserialize)]
pub struct CreatePlayerRequest {
    pub name: String,
    #[serde(default = "default_model")]
    pub skin_model: String,
}

fn default_model() -> String {
    "classic".to_string()
}

/// POST /api/players - 创建角色
#[instrument(skip_all, fields(name = %req.name), level = "debug")]
pub async fn create_player_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePlayerRequest>,
) -> ApiResult<JsonResponse<PlayerDto>> {
    let tok = bearer_token(&state, &headers).await?;
    let name = req.name.trim();
    validate_player_name(name)?;
    if req.skin_model != "classic" && req.skin_model != "slim" {
        return Err(ApiError::bad_request("Invalid skin model"));
    }

    // 角色数量上限
    let count = count_players_by_user(&state.pool, &tok.user_id)
        .await
        .map_err(db_err)?;
    if count >= state.config.auth.max_players_per_user as i64 {
        warn!(user_id = %tok.user_id, max = state.config.auth.max_players_per_user, "max players reached");
        return Err(ApiError::bad_request(format!(
            "Maximum {} players reached",
            state.config.auth.max_players_per_user
        )));
    }

    // 角色名全局唯一
    if crate::core::db::get_player_by_name(&state.pool, name)
        .await
        .map_err(db_err)?
        .is_some()
    {
        warn!(name = %name, "player name already exists");
        return Err(ApiError::bad_request("Player name already exists"));
    }

    let player_id = match state.config.auth.player_uuid_generation {
        UuidGeneration::Offline => offline_uuid(name),
        UuidGeneration::Random => random_uuid(),
    };
    create_player(
        &state.pool,
        &player_id,
        name,
        &tok.user_id,
        None,
        None,
        &req.skin_model,
    )
    .await
    .map_err(db_err)?;

    info!(player_id = %player_id, name = %name, user_id = %tok.user_id, "player created");

    let player = get_player_by_id(&state.pool, &player_id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| ApiError::internal("failed to read back player"))?;
    Ok(JsonResponse(PlayerDto::from(player)))
}

/// DELETE /api/players/{id} - 删除角色
#[instrument(skip_all, fields(id = %id), level = "debug")]
pub async fn delete_player_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let tok = bearer_token(&state, &headers).await?;
    let player = get_player_by_id(&state.pool, &id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            debug!(id = %id, "delete player: not found");
            ApiError::not_found("Player not found")
        })?;
    if player.user_id != tok.user_id {
        warn!(id = %id, "delete player: not owned by token user");
        return Err(ApiError::not_found("Player not found"));
    }
    delete_player(&state.pool, &id).await.map_err(db_err)?;
    info!(id = %id, "player deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// 请求体:切换皮肤模型
#[derive(Debug, Deserialize)]
pub struct UpdateSkinModelRequest {
    pub model: String,
}

/// PUT /api/players/{id}/skin-model - 切换皮肤模型
#[instrument(skip_all, fields(id = %id, model = %req.model), level = "debug")]
pub async fn update_skin_model_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateSkinModelRequest>,
) -> ApiResult<StatusCode> {
    let tok = bearer_token(&state, &headers).await?;
    let player = get_player_by_id(&state.pool, &id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            debug!(id = %id, "update skin model: player not found");
            ApiError::not_found("Player not found")
        })?;
    if player.user_id != tok.user_id {
        warn!(id = %id, "update skin model: not owned by token user");
        return Err(ApiError::not_found("Player not found"));
    }
    if req.model != "classic" && req.model != "slim" {
        warn!(id = %id, model = %req.model, "invalid skin model");
        return Err(ApiError::bad_request("Invalid skin model"));
    }
    update_skin_model(&state.pool, &id, &req.model)
        .await
        .map_err(db_err)?;
    info!(id = %id, model = %req.model, "skin model updated");
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/me 响应体
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub username: String,
    pub preferred_language: String,
    pub players: Vec<PlayerDto>,
}
