//! SQLite 数据层:Schema 初始化与用户/角色/令牌查询

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;

mod models;
pub use models::{Player, TextureKind, Token, User};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id                TEXT PRIMARY KEY,
    username          TEXT NOT NULL UNIQUE,
    password_hash     TEXT NOT NULL,
    preferred_language TEXT NOT NULL DEFAULT 'zh_CN'
);

CREATE TABLE IF NOT EXISTS players (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    skin_hash   TEXT,
    cape_hash   TEXT,
    skin_model  TEXT NOT NULL DEFAULT 'classic',
    CHECK (skin_model IN ('classic', 'slim'))
);

CREATE TABLE IF NOT EXISTS tokens (
    access_token TEXT PRIMARY KEY,
    client_token TEXT NOT NULL,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    player_id    TEXT REFERENCES players(id) ON DELETE CASCADE,
    issued_at    INTEGER NOT NULL,
    expires_at   INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_players_user ON players(user_id);
CREATE INDEX IF NOT EXISTS idx_tokens_user ON tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_tokens_player ON tokens(player_id);
"#;

/// 打开(必要时创建)SQLite 数据库并初始化表结构
pub async fn init_db(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create dir: {}", parent.display()))?;
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .context("failed to open sqlite database")?;
    sqlx::query(SCHEMA)
        .execute(&pool)
        .await
        .context("failed to init schema")?;
    Ok(pool)
}

// ---- 用户 ----

pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn get_user_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn create_user(
    pool: &SqlitePool,
    id: &str,
    username: &str,
    password_hash: &str,
    preferred_language: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, preferred_language) VALUES (?, ?, ?, ?)",
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .bind(preferred_language)
    .execute(pool)
    .await?;
    Ok(())
}

// ---- 角色 ----

pub async fn get_player_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Player>> {
    Ok(
        sqlx::query_as::<_, Player>("SELECT * FROM players WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn get_player_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Player>> {
    Ok(
        sqlx::query_as::<_, Player>("SELECT * FROM players WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn get_players_by_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<Player>> {
    Ok(
        sqlx::query_as::<_, Player>("SELECT * FROM players WHERE user_id = ? ORDER BY name")
            .bind(user_id)
            .fetch_all(pool)
            .await?,
    )
}

pub async fn create_player(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    user_id: &str,
    skin_hash: Option<&str>,
    cape_hash: Option<&str>,
    skin_model: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO players (id, name, user_id, skin_hash, cape_hash, skin_model) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(user_id)
    .bind(skin_hash)
    .bind(cape_hash)
    .bind(skin_model)
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新角色的材质(hash 为 None 表示清除)
pub async fn update_player_texture(
    pool: &SqlitePool,
    player_id: &str,
    kind: TextureKind,
    hash: Option<&str>,
) -> Result<()> {
    let column = match kind {
        TextureKind::Skin => "skin_hash",
        TextureKind::Cape => "cape_hash",
    };
    let sql = format!("UPDATE players SET {} = ? WHERE id = ?", column);
    sqlx::query(&sql)
        .bind(hash)
        .bind(player_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 批量按名称查询角色(保持输入顺序,不存在的跳过)
pub async fn get_players_by_names(pool: &SqlitePool, names: &[String]) -> Result<Vec<Player>> {
    let mut players = Vec::new();
    for name in names {
        if let Some(p) = get_player_by_name(pool, name).await? {
            players.push(p);
        }
    }
    Ok(players)
}

// ---- 令牌 ----

pub async fn get_token(pool: &SqlitePool, access_token: &str) -> Result<Option<Token>> {
    Ok(
        sqlx::query_as::<_, Token>("SELECT * FROM tokens WHERE access_token = ?")
            .bind(access_token)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn create_token(
    pool: &SqlitePool,
    access_token: &str,
    client_token: &str,
    user_id: &str,
    player_id: Option<&str>,
    issued_at: i64,
    expires_at: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO tokens (access_token, client_token, user_id, player_id, issued_at, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(access_token)
    .bind(client_token)
    .bind(user_id)
    .bind(player_id)
    .bind(issued_at)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_token(pool: &SqlitePool, access_token: &str) -> Result<()> {
    sqlx::query("DELETE FROM tokens WHERE access_token = ?")
        .bind(access_token)
        .execute(pool)
        .await?;
    Ok(())
}

/// 吊销用户的所有令牌(signout)
pub async fn delete_tokens_by_user(pool: &SqlitePool, user_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM tokens WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 删除用户的指定角色令牌(角色删除时清理)
pub async fn delete_tokens_by_player(pool: &SqlitePool, player_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM tokens WHERE player_id = ?")
        .bind(player_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 限制用户令牌数量:超限时按颁发时间吊销最旧的令牌(规范建议,如上限 10)
pub async fn enforce_token_limit(pool: &SqlitePool, user_id: &str, max: usize) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tokens WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let excess = count - max as i64;
    if excess <= 0 {
        return Ok(());
    }
    sqlx::query(
        "DELETE FROM tokens WHERE access_token IN (
            SELECT access_token FROM tokens WHERE user_id = ?
            ORDER BY issued_at ASC, access_token ASC
            LIMIT ?
        )",
    )
    .bind(user_id)
    .bind(excess)
    .execute(pool)
    .await?;
    Ok(())
}

/// 删除所有过期令牌,返回删除行数(启动时 + 定期清理调用)
pub async fn delete_expired_tokens(pool: &SqlitePool, now: i64) -> Result<u64> {
    Ok(sqlx::query("DELETE FROM tokens WHERE expires_at <= ?")
        .bind(now)
        .execute(pool)
        .await?
        .rows_affected())
}
