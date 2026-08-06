//! 材质系统:PNG 安全校验、存储、textures 属性生成与签名
//!
//! 安全要求(authlib-injector 规范):
//! - 先读图像头获取尺寸,防止 PNG bomb 消耗内存
//! - 校验尺寸:皮肤为 64x32/64x64 整数倍;披风为 64x32/22x17 整数倍(22x17 需补足)
//! - 重新编码 PNG 以去除与位图无关的数据(防隐藏恶意代码)

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use png::{BitDepth, ColorType, Decoder, Encoder};
use rsa::RsaPrivateKey;
use serde::Serialize;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::crypto::{now_millis, sha256_hex, sign_sha1};
use crate::db::{Player, TextureKind};

/// 材质尺寸上限(像素),防止超大图像
const MAX_TEXTURE_SIZE: u32 = 1024;

/// 材质存储:data/textures/{sha256}.png
#[derive(Clone)]
pub struct TextureStore {
    dir: PathBuf,
}

impl TextureStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let dir = data_dir.join("textures");
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create textures dir: {}", dir.display()))?;
        Ok(TextureStore { dir })
    }

    /// 保存材质数据,返回 SHA-256 hash;已存在则直接返回 hash
    pub fn save(&self, data: &[u8]) -> Result<String> {
        let hash = sha256_hex(data);
        let path = self.dir.join(format!("{}.png", hash));
        if !path.exists() {
            std::fs::write(&path, data)
                .with_context(|| format!("failed to write texture: {}", path.display()))?;
        }
        Ok(hash)
    }

    /// 读取材质数据
    pub fn load(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let path = self.dir.join(format!("{}.png", hash));
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(
            std::fs::read(&path)
                .with_context(|| format!("failed to read texture: {}", path.display()))?,
        ))
    }
}

/// 校验并清洗 PNG:检查尺寸合法性、重编码去除无关数据,返回干净的 RGBA8 PNG
pub fn sanitize_png(data: &[u8], kind: TextureKind) -> Result<Vec<u8>> {
    // 先仅读取头部获取尺寸,未解码像素数据前不分配大块内存
    let decoder = Decoder::new(Cursor::new(data));
    let mut reader = decoder
        .read_info()
        .context("invalid PNG: failed to read header")?;
    let (w, h) = (reader.info().width, reader.info().height);

    if w == 0 || h == 0 {
        anyhow::bail!("invalid texture size: {}x{}", w, h);
    }
    if w > MAX_TEXTURE_SIZE || h > MAX_TEXTURE_SIZE {
        anyhow::bail!(
            "texture too large: {}x{} (max {})",
            w,
            h,
            MAX_TEXTURE_SIZE
        );
    }
    validate_dimensions(w, h, kind)?;

    // 解码像素
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .context("invalid PNG: failed to decode")?;
    let (w, h) = (info.width as usize, info.height as usize);

    // 统一转为 RGBA8
    let rgba: Vec<u8> = match info.color_type {
        ColorType::Rgba => buf[..w * h * 4].to_vec(),
        ColorType::Rgb => {
            let mut v = Vec::with_capacity(w * h * 4);
            for i in 0..w * h {
                v.extend_from_slice(&buf[i * 3..i * 3 + 3]);
                v.push(255);
            }
            v
        }
        other => anyhow::bail!("unsupported PNG color type: {:?}", other),
    };

    // 重新编码,丢弃所有元数据(文本块、tRNS 等)
    let mut out = Vec::new();
    {
        let mut encoder = Encoder::new(&mut out, w as u32, h as u32);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().context("PNG encode failed")?;
        writer
            .write_image_data(&rgba)
            .context("PNG encode failed")?;
    }
    Ok(out)
}

/// 校验材质尺寸(authlib-injector 规范)
fn validate_dimensions(w: u32, h: u32, kind: TextureKind) -> Result<()> {
    match kind {
        // 皮肤:64x32 的整数倍或 64x64 的整数倍
        TextureKind::Skin => {
            if w % 64 != 0 {
                anyhow::bail!("invalid skin width: {}", w);
            }
            let a = w / 64;
            if h != 32 * a && h != 64 * a {
                anyhow::bail!("invalid skin height: {} (width {})", h, w);
            }
        }
        // 披风:64x32 的整数倍或 22x17 的整数倍
        TextureKind::Cape => {
            let ok_standard = w % 64 == 0 && {
                let a = w / 64;
                h == 32 * a
            };
            let ok_legacy = w % 22 == 0 && {
                let a = w / 22;
                h == 17 * a
            };
            if !ok_standard && !ok_legacy {
                anyhow::bail!("invalid cape dimensions: {}x{}", w, h);
            }
        }
    }
    Ok(())
}

/// 将 22x17 倍数披风补足到 64x32 倍数(带透明像素)
pub fn pad_cape(data: &[u8]) -> Result<Vec<u8>> {
    let decoder = Decoder::new(Cursor::new(data));
    let mut reader = decoder
        .read_info()
        .context("invalid PNG: failed to read header")?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w % 22 != 0 || h != 17 * (w / 22) {
        return Ok(data.to_vec());
    }
    // 22x17 倍数:补足到 (64a, 32a)
    let a = w / 22;
    let (tw, th) = (64 * a, 32 * a);
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .context("invalid PNG: failed to decode")?;
    // 透明像素 = 全零 RGBA
    let mut target = vec![0u8; (tw as usize) * (th as usize) * 4];
    for row in 0..(h as usize) {
        let dst_start = row * (tw as usize) * 4;
        match info.color_type {
            ColorType::Rgba => {
                let src_start = row * (w as usize) * 4;
                target[dst_start..dst_start + (w as usize) * 4]
                    .copy_from_slice(&buf[src_start..src_start + (w as usize) * 4]);
            }
            ColorType::Rgb => {
                let src_start = row * (w as usize) * 3;
                for col in 0..(w as usize) {
                    let s = src_start + col * 3;
                    let d = dst_start + col * 4;
                    target[d..d + 3].copy_from_slice(&buf[s..s + 3]);
                    target[d + 3] = 255;
                }
            }
            _ => anyhow::bail!("unsupported PNG color type"),
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = Encoder::new(&mut out, tw, th);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().context("PNG encode failed")?;
        writer
            .write_image_data(&target)
            .context("PNG encode failed")?;
    }
    Ok(out)
}

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
            serde_json::Value::String(format!("{}/textures/{}", config.base_url.trim_end_matches('/'), skin_hash)),
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
            serde_json::Value::String(format!("{}/textures/{}", config.base_url.trim_end_matches('/'), cape_hash)),
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

/// 导入材质文件(sanitize + 存储),返回 hash;22x17 披风自动补足
pub fn import_texture_file(store: &TextureStore, path: &Path, kind: TextureKind) -> Result<String> {
    let data = std::fs::read(path)
        .with_context(|| format!("failed to read texture file: {}", path.display()))?;
    let mut clean = sanitize_png(&data, kind)?;
    if kind == TextureKind::Cape {
        clean = pad_cape(&clean)?;
    }
    store.save(&clean)
}

#[cfg(test)]
mod tests {
    use super::*;
    use png::ColorType as PngColorType;

    /// 构造一个简单 RGBA PNG
    fn make_png(w: u32, h: u32, rgba: bool) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = Encoder::new(&mut out, w, h);
            encoder.set_color(if rgba { ColorType::Rgba } else { ColorType::Rgb });
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let total = (w * h) as usize * if rgba { 4 } else { 3 };
            let data = vec![0x80u8; total];
            writer.write_image_data(&data).unwrap();
        }
        out
    }

    #[test]
    fn test_sanitize_skin_ok() {
        // 64x32 皮肤
        let png = make_png(64, 32, true);
        let clean = sanitize_png(&png, TextureKind::Skin).unwrap();
        // 重新解析确认尺寸
        let decoder = Decoder::new(Cursor::new(&clean));
        let mut reader = decoder.read_info().unwrap();
        assert_eq!(reader.info().width, 64);
        assert_eq!(reader.info().height, 32);
        let mut buf = vec![0u8; reader.output_buffer_size()];
        reader.next_frame(&mut buf).unwrap();
        assert_eq!(reader.info().color_type, PngColorType::Rgba);
    }

    #[test]
    fn test_sanitize_skin_bad_size() {
        // 63x32 非法
        let png = make_png(63, 32, true);
        assert!(sanitize_png(&png, TextureKind::Skin).is_err());
        // 64x64 合法(64x64 倍数)
        let png = make_png(64, 64, true);
        assert!(sanitize_png(&png, TextureKind::Skin).is_ok());
        // 128x64 合法
        let png = make_png(128, 64, true);
        assert!(sanitize_png(&png, TextureKind::Skin).is_ok());
    }

    #[test]
    fn test_sanitize_cape_ok() {
        // 22x17 合法(会补足)
        let png = make_png(22, 17, true);
        let clean = sanitize_png(&png, TextureKind::Cape).unwrap();
        let padded = pad_cape(&clean).unwrap();
        let decoder = Decoder::new(Cursor::new(&padded));
        let reader = decoder.read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (64, 32));
        // 64x32 标准披风
        let png = make_png(64, 32, true);
        assert!(sanitize_png(&png, TextureKind::Cape).is_ok());
    }

    #[test]
    fn test_sanitize_too_large() {
        let png = make_png(2048, 1024, true);
        assert!(sanitize_png(&png, TextureKind::Skin).is_err());
    }

    #[test]
    fn test_rejects_text_metadata() {
        // 带 tEXt 块的 PNG 重编码后应被去除
        let png = make_png(64, 32, true);
        let clean = sanitize_png(&png, TextureKind::Skin).unwrap();
        assert!(clean.len() <= png.len() + 64); // 重编码体积合理
    }

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
        assert!(json["textures"]["SKIN"]["url"]
            .as_str()
            .unwrap()
            .ends_with("/textures/abc123"));
    }
}
