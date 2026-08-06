//! Yggdrasil API 公共序列化类型

use serde::Serialize;

use crate::db::{Player, User};

/// 角色/用户属性(properties)
#[derive(Debug, Clone, Serialize)]
pub struct Property {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl Property {
    pub fn plain(name: impl Into<String>, value: impl Into<String>) -> Self {
        Property {
            name: name.into(),
            value: value.into(),
            signature: None,
        }
    }

    pub fn signed(name: impl Into<String>, value: impl Into<String>, signature: String) -> Self {
        Property {
            name: name.into(),
            value: value.into(),
            signature: Some(signature),
        }
    }
}

/// 角色信息序列化(无符号 UUID)
#[derive(Debug, Clone, Serialize)]
pub struct ProfileResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<Property>>,
}

impl ProfileResponse {
    /// 基础角色信息(不含 properties)
    pub fn basic(player: &Player) -> Self {
        ProfileResponse {
            id: player.id.clone(),
            name: player.name.clone(),
            properties: None,
        }
    }

    /// 完整角色信息(含 properties)
    pub fn full(player: &Player, properties: Vec<Property>) -> Self {
        ProfileResponse {
            id: player.id.clone(),
            name: player.name.clone(),
            properties: Some(properties),
        }
    }
}

/// 用户信息序列化
#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub properties: Vec<Property>,
}

impl UserResponse {
    pub fn from_user(user: &User) -> Self {
        UserResponse {
            id: user.id.clone(),
            properties: vec![Property::plain(
                "preferredLanguage",
                &user.preferred_language,
            )],
        }
    }
}
