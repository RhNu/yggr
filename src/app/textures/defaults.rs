//! 内置默认皮肤:steve(classic)与 alex(slim),作为无皮肤时的回退

use anyhow::Result;
use tracing::instrument;

use crate::core::db::TextureKind;

use super::process::sanitize_png;
use super::store::TextureStore;

const STEVE_PNG: &[u8] = include_bytes!("defaults/steve.png");
const ALEX_PNG: &[u8] = include_bytes!("defaults/alex.png");

/// 内置默认皮肤哈希
#[derive(Clone)]
pub struct DefaultSkins {
    classic: String,
    slim: String,
}

impl DefaultSkins {
    /// 将内置皮肤导入材质存储,返回哈希
    #[instrument(skip_all, level = "debug")]
    pub fn init(store: &TextureStore) -> Result<Self> {
        let classic = sanitize_png(STEVE_PNG, TextureKind::Skin)?;
        let slim = sanitize_png(ALEX_PNG, TextureKind::Skin)?;
        let classic_hash = store.save(&classic)?;
        let slim_hash = store.save(&slim)?;
        Ok(DefaultSkins {
            classic: classic_hash,
            slim: slim_hash,
        })
    }

    /// 按模型返回默认皮肤哈希
    pub fn hash_for(&self, model: &str) -> &str {
        if model == "slim" {
            &self.slim
        } else {
            &self.classic
        }
    }

    /// classic 模型哈希
    pub fn classic_hash(&self) -> &str {
        &self.classic
    }

    /// slim 模型哈希
    pub fn slim_hash(&self) -> &str {
        &self.slim
    }
}

#[cfg(test)]
impl DefaultSkins {
    /// 测试用:直接构造,不导入真实皮肤文件
    pub fn test_new(classic: &str, slim: &str) -> Self {
        DefaultSkins {
            classic: classic.to_string(),
            slim: slim.to_string(),
        }
    }
}
