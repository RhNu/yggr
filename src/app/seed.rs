//! 种子用户初始化:启动时从 seed.toml 创建用户与角色

use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::{info, warn};

use crate::app::textures::{TextureStore, import_texture_file};
use crate::core::config::{Config, UuidGeneration};
use crate::core::crypto::{hash_password, offline_uuid, random_uuid};
use crate::core::db::{
    TextureKind, create_player, create_user, get_player_by_name, get_user_by_username,
};

#[derive(Debug, Deserialize)]
pub struct SeedConfig {
    #[serde(default)]
    pub users: Vec<SeedUser>,
}

#[derive(Debug, Deserialize)]
pub struct SeedUser {
    pub username: String,
    pub password: String,
    #[serde(default = "default_language")]
    pub preferred_language: String,
    #[serde(default)]
    pub players: Vec<SeedPlayer>,
}

fn default_language() -> String {
    "zh_CN".to_string()
}

#[derive(Debug, Deserialize)]
pub struct SeedPlayer {
    pub name: String,
    /// 可选:直接指定角色 UUID(与 Mojang 官方账号兼容迁移)
    pub uuid: Option<String>,
    /// classic | slim
    #[serde(default = "default_model")]
    pub skin_model: String,
    /// 可选:皮肤 PNG 文件路径
    pub skin: Option<PathBuf>,
    /// 可选:披风 PNG 文件路径
    pub cape: Option<PathBuf>,
}

fn default_model() -> String {
    "classic".to_string()
}

/// 应用种子配置:已存在的用户名/角色名跳过
pub async fn apply_seed(config: &Config, pool: &SqlitePool, store: &TextureStore) -> Result<()> {
    let Some(seed_path) = &config.seed.file else {
        return Ok(());
    };
    if !seed_path.exists() {
        warn!("seed file not found: {}, skipping", seed_path.display());
        return Ok(());
    }
    let content = std::fs::read_to_string(seed_path)
        .with_context(|| format!("failed to read seed file: {}", seed_path.display()))?;
    let seed: SeedConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse seed file: {}", seed_path.display()))?;

    for user_cfg in &seed.users {
        if get_user_by_username(pool, &user_cfg.username)
            .await?
            .is_some()
        {
            info!("seed: user {} already exists, skipping", user_cfg.username);
            continue;
        }
        let password_hash = hash_password(&user_cfg.password)?;
        let user_id = random_uuid();
        create_user(
            pool,
            &user_id,
            &user_cfg.username,
            &password_hash,
            &user_cfg.preferred_language,
        )
        .await?;
        info!("seed: created user {}", user_cfg.username);

        for player_cfg in &user_cfg.players {
            if get_player_by_name(pool, &player_cfg.name).await?.is_some() {
                warn!("seed: player {} already exists, skipping", player_cfg.name);
                continue;
            }
            if player_cfg.skin_model != "classic" && player_cfg.skin_model != "slim" {
                anyhow::bail!(
                    "seed: invalid skin_model {:?} for player {} (classic|slim)",
                    player_cfg.skin_model,
                    player_cfg.name
                );
            }

            let player_id = match &player_cfg.uuid {
                Some(u) => normalize_uuid(u)?,
                None => match config.auth.player_uuid_generation {
                    UuidGeneration::Offline => offline_uuid(&player_cfg.name),
                    UuidGeneration::Random => random_uuid(),
                },
            };

            let skin_hash = match &player_cfg.skin {
                Some(path) => Some(import_texture_file(store, path, TextureKind::Skin)?),
                None => None,
            };
            let cape_hash = match &player_cfg.cape {
                Some(path) => Some(import_texture_file(store, path, TextureKind::Cape)?),
                None => None,
            };

            create_player(
                pool,
                &player_id,
                &player_cfg.name,
                &user_id,
                skin_hash.as_deref(),
                cape_hash.as_deref(),
                &player_cfg.skin_model,
            )
            .await?;
            info!("seed: created player {} ({})", player_cfg.name, player_id);
        }
    }
    Ok(())
}

/// 校验 seed 文件中的 UUID 格式(有则必须是合法 UUID,自动转无符号)
pub fn normalize_uuid(u: &str) -> Result<String> {
    let compact = u.to_ascii_lowercase().replace('-', "");
    if compact.len() != 32 || !compact.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("invalid uuid: {}", u);
    }
    Ok(compact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_uuid() {
        assert_eq!(
            normalize_uuid("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "550e8400e29b41d4a716446655440000"
        );
        assert!(normalize_uuid("not-a-uuid").is_err());
    }
}
