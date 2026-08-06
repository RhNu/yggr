//! textures 属性值生成与签名(供 hasJoined/profile 响应使用)

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rsa::RsaPrivateKey;
use serde::Serialize;

use crate::core::config::Config;
use crate::core::crypto::{now_millis, sign_sha1};
use crate::core::db::Player;

/// textures 属性值结构
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TexturesPayload<'a> {
    timestamp: i64,
    profile_id: String,
    profile_name: &'a str,
    textures: serde_json::Map<String, serde_json::Value>,
}

/// 生成 Base64 编码的 textures 属性值(含 timestamp/profileId/profileName)
pub fn build_textures_value(config: &Config, player: &Player) -> Result<String> {
    let mut textures = serde_json::Map::new();
    if let Some(skin_hash) = &player.skin_hash {
        let mut skin = serde_json::Map::new();
        skin.insert(
            "url".to_string(),
            serde_json::Value::String(format!(
                "{}/service/textures/{}",
                config.base_url.trim_end_matches('/'),
                skin_hash
            )),
        );
        if player.skin_model == "slim" {
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "model".to_string(),
                serde_json::Value::String("slim".to_string()),
            );
            skin.insert("metadata".to_string(), serde_json::Value::Object(metadata));
        }
        textures.insert("SKIN".to_string(), serde_json::Value::Object(skin));
    }
    if let Some(cape_hash) = &player.cape_hash {
        let mut cape = serde_json::Map::new();
        cape.insert(
            "url".to_string(),
            serde_json::Value::String(format!(
                "{}/service/textures/{}",
                config.base_url.trim_end_matches('/'),
                cape_hash
            )),
        );
        textures.insert("CAPE".to_string(), serde_json::Value::Object(cape));
    }

    let payload = TexturesPayload {
        timestamp: now_millis(),
        profile_id: player.id.clone(),
        profile_name: &player.name,
        textures,
    };
    let json = serde_json::to_string(&payload).context("failed to serialize textures payload")?;
    Ok(BASE64.encode(json.as_bytes()))
}

/// 计算 textures 属性值的 SHA1withRSA 签名(Base64)
pub fn sign_textures_value(key: &RsaPrivateKey, value: &str) -> Result<String> {
    let sig = sign_sha1(key, value.as_bytes())?;
    Ok(BASE64.encode(sig))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textures_value() {
        let config = Config::default();
        let player = Player {
            id: "5627dd98e6be3c21b8a8e92344183641".to_string(),
            name: "Steve".to_string(),
            user_id: "u1".to_string(),
            skin_hash: Some("abc123".to_string()),
            cape_hash: None,
            skin_model: "slim".to_string(),
        };
        let value = build_textures_value(&config, &player).unwrap();
        let decoded = BASE64.decode(&value).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(json["profileName"], "Steve");
        assert_eq!(json["textures"]["SKIN"]["metadata"]["model"], "slim");
        assert!(
            json["textures"]["SKIN"]["url"]
                .as_str()
                .unwrap()
                .ends_with("/service/textures/abc123")
        );
    }
}
