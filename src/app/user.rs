//! 用户初始化:启动时从 user.toml 创建允许登录的用户

use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::core::config::Config;
use crate::core::crypto::{hash_password, random_uuid};
use crate::core::db::{create_user, get_user_by_username};

#[derive(Debug, Deserialize)]
pub struct UserFile {
    #[serde(default)]
    pub users: Vec<UserEntry>,
}

#[derive(Debug, Deserialize)]
pub struct UserEntry {
    pub username: String,
    pub password: String,
    #[serde(default = "default_language")]
    pub preferred_language: String,
}

fn default_language() -> String {
    "zh_CN".to_string()
}

/// 应用用户配置:已存在的用户名跳过
pub async fn apply_users(config: &Config, pool: &SqlitePool) -> Result<()> {
    let Some(user_path) = &config.user.file else {
        return Ok(());
    };
    if !user_path.exists() {
        warn!("user file not found: {}, skipping", user_path.display());
        return Ok(());
    }
    let content = std::fs::read_to_string(user_path)
        .with_context(|| format!("failed to read user file: {}", user_path.display()))?;
    let user_file: UserFile = toml::from_str(&content)
        .with_context(|| format!("failed to parse user file: {}", user_path.display()))?;

    for user_cfg in &user_file.users {
        if get_user_by_username(pool, &user_cfg.username)
            .await?
            .is_some()
        {
            info!("user {} already exists, skipping", user_cfg.username);
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
        info!("created user {}", user_cfg.username);
    }
    Ok(())
}
