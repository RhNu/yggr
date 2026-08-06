//! 材质存储:data/textures/{sha256}.png 的落盘与读取

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::instrument;

use crate::core::crypto::sha256_hex;
use crate::core::db::TextureKind;

use super::process::{pad_cape, sanitize_png};

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
    #[instrument(skip(self, data), fields(hash), level = "trace")]
    pub fn save(&self, data: &[u8]) -> Result<String> {
        let hash = sha256_hex(data);
        let path = self.dir.join(format!("{}.png", hash));
        if !path.exists() {
            std::fs::write(&path, data)
                .with_context(|| format!("failed to write texture: {}", path.display()))?;
        }
        tracing::Span::current().record("hash", &hash);
        Ok(hash)
    }

    /// 读取材质数据
    #[instrument(skip(self), fields(hash = %hash), level = "trace")]
    pub fn load(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let path = self.dir.join(format!("{}.png", hash));
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read(&path).with_context(|| {
            format!("failed to read texture: {}", path.display())
        })?))
    }
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
