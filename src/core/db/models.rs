//! 数据模型:SQLite 表结构对应的 Rust 类型

use sqlx::FromRow;

/// 用户(users 表)
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub preferred_language: String,
}

/// 游戏角色(players 表)
#[derive(Debug, Clone, FromRow)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub skin_hash: Option<String>,
    pub cape_hash: Option<String>,
    pub skin_model: String,
}

/// 访问令牌(tokens 表)
#[derive(Debug, Clone, FromRow)]
pub struct Token {
    pub access_token: String,
    pub client_token: String,
    pub user_id: String,
    pub player_id: Option<String>,
    pub issued_at: i64,
    pub expires_at: i64,
}

/// 材质类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureKind {
    Skin,
    Cape,
}

impl TextureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextureKind::Skin => "skin",
            TextureKind::Cape => "cape",
        }
    }
}
