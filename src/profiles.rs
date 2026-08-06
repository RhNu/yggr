//! /api/* 角色 API:
//! - POST /api/profiles/minecraft 按名称批量查询角色
//! - PUT/DELETE /api/user/profile/{uuid}/{skin|cape} 材质上传/清除

use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use crate::auth::bearer_token;
use crate::db::{
    get_player_by_id, get_players_by_names, update_player_texture, TextureKind,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::textures::{pad_cape, sanitize_png};
use crate::types::ProfileResponse;

/// 单次批量查询的最大角色数(防 CC 攻击)
const MAX_PROFILES_PER_REQUEST: usize = 100;

fn db_err(e: anyhow::Error) -> ApiError {
    ApiError::internal(e.to_string())
}

fn sqlx_err(e: sqlx::Error) -> ApiError {
    ApiError::internal(e.to_string())
}

fn internal(e: anyhow::Error) -> ApiError {
    ApiError::internal(e.to_string())
}

/// POST /api/profiles/minecraft
pub async fn batch_profiles(
    State(state): State<AppState>,
    Json(names): Json<Vec<String>>,
) -> ApiResult<Json<Vec<ProfileResponse>>> {
    if names.is_empty() {
        return Ok(Json(Vec::new()));
    }
    if names.len() > MAX_PROFILES_PER_REQUEST {
        return Err(ApiError::bad_request(format!(
            "Too many profiles requested (max {})",
            MAX_PROFILES_PER_REQUEST
        )));
    }
    let players = get_players_by_names(&state.pool, &names)
        .await
        .map_err(db_err)?;
    Ok(Json(
        players.iter().map(ProfileResponse::basic).collect(),
    ))
}

// ---- 材质上传/清除 ----

/// 材质路由的 Bearer 认证中间件(multipart 提取器在 handler 内执行前会先解析,
/// 因此认证检查需要放在路由层,保证未认证请求返回 401 而不是 400)
pub async fn require_bearer(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    bearer_token(&state, &headers).await?;
    Ok(next.run(req).await)
}

/// PUT /api/user/profile/{uuid}/skin
pub async fn upload_skin(
    state: State<AppState>,
    headers: HeaderMap,
    path: Path<String>,
    multipart: Multipart,
) -> ApiResult<StatusCode> {
    upload_texture(state, headers, path, TextureKind::Skin, multipart).await
}

/// PUT /api/user/profile/{uuid}/cape
pub async fn upload_cape(
    state: State<AppState>,
    headers: HeaderMap,
    path: Path<String>,
    multipart: Multipart,
) -> ApiResult<StatusCode> {
    upload_texture(state, headers, path, TextureKind::Cape, multipart).await
}

/// PUT /api/user/profile/{uuid}/{textureType}
async fn upload_texture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(uuid): Path<String>,
    kind: TextureKind,
    mut multipart: Multipart,
) -> ApiResult<StatusCode> {
    let tok = bearer_token(&state, &headers).await?;

    let player = get_player_by_id(&state.pool, &uuid)
        .await
        .map_err(db_err)?
        .ok_or_else(|| ApiError::not_found("Unknown profile"))?;
    // 角色必须属于令牌用户
    if player.user_id != tok.user_id {
        return Err(ApiError::not_found("Unknown profile"));
    }

    // 解析 multipart:model(皮肤模型)、file(图像)
    let mut model: Option<String> = None;
    let mut file: Option<(Option<String>, Vec<u8>)> = None; // (content_type, data)
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?
    {
        match field.name().unwrap_or("") {
            "model" => {
                model = Some(field.text().await.map_err(|e| ApiError::bad_request(e.to_string()))?);
            }
            "file" => {
                let content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
                    .to_vec();
                file = Some((content_type, data));
            }
            _ => {}
        }
    }
    let Some((content_type, data)) = file else {
        return Err(ApiError::bad_request("Missing file field"));
    };
    // 规范:file 的 Content-Type 须为 image/png
    if let Some(ct) = &content_type {
        if ct != "image/png" {
            return Err(ApiError::bad_request("File must be image/png"));
        }
    }

    // PNG 校验与清洗(防 PNG bomb、去除无关数据)
    let mut clean = sanitize_png(&data, kind).map_err(|e| ApiError::bad_request(e.to_string()))?;
    // 22x17 披风补足到 64x32
    if kind == TextureKind::Cape {
        clean = pad_cape(&clean).map_err(internal)?;
    }
    let hash = state
        .store
        .save(&clean)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // 皮肤模型更新(slim/classic)
    if kind == TextureKind::Skin {
        if let Some(m) = &model {
            let m = m.trim();
            if !m.is_empty() && m != "slim" && m != "classic" {
                return Err(ApiError::bad_request("Invalid model"));
            }
            if m == "slim" {
                sqlx::query("UPDATE players SET skin_model = 'slim' WHERE id = ?")
                    .bind(&player.id)
                    .execute(&state.pool)
                    .await
                    .map_err(sqlx_err)?;
            } else if m == "classic" {
                sqlx::query("UPDATE players SET skin_model = 'classic' WHERE id = ?")
                    .bind(&player.id)
                    .execute(&state.pool)
                    .await
                    .map_err(sqlx_err)?;
            }
        }
    }

    update_player_texture(&state.pool, &player.id, kind, Some(&hash))
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/user/profile/{uuid}/skin
pub async fn delete_skin(
    state: State<AppState>,
    headers: HeaderMap,
    path: Path<String>,
) -> ApiResult<StatusCode> {
    delete_texture(state, headers, path, TextureKind::Skin).await
}

/// DELETE /api/user/profile/{uuid}/cape
pub async fn delete_cape(
    state: State<AppState>,
    headers: HeaderMap,
    path: Path<String>,
) -> ApiResult<StatusCode> {
    delete_texture(state, headers, path, TextureKind::Cape).await
}

/// DELETE /api/user/profile/{uuid}/{textureType}
async fn delete_texture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(uuid): Path<String>,
    kind: TextureKind,
) -> ApiResult<StatusCode> {
    let tok = bearer_token(&state, &headers).await?;

    let player = get_player_by_id(&state.pool, &uuid)
        .await
        .map_err(db_err)?
        .ok_or_else(|| ApiError::not_found("Unknown profile"))?;
    if player.user_id != tok.user_id {
        return Err(ApiError::not_found("Unknown profile"));
    }
    update_player_texture(&state.pool, &player.id, kind, None)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /textures/{hash} — 材质文件服务
pub async fn texture_file(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let Some(data) = state
        .store
        .load(&hash)
        .map_err(|e| ApiError::internal(e.to_string()))?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok((
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        data,
    )
        .into_response())
}
