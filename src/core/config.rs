//! 服务配置:解析 config.toml(分区结构)

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// 角色 UUID 生成策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UuidGeneration {
    /// 离线兼容:MD5("OfflinePlayer:" + name),可与离线服平滑迁移
    #[default]
    Offline,
    /// 随机 UUID v4
    Random,
}

/// [server] 服务器基本配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// 服务器名称,显示在 meta.serverName
    pub name: String,
    /// 外部访问基础 URL,用于生成材质 URL
    pub base_url: String,
    /// 监听地址
    pub listen: String,
    /// 额外的材质域名白名单规则(默认包含 base_url 的域名)
    pub skin_domains: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "My Yggdrasil".to_string(),
            base_url: "http://127.0.0.1:8080".to_string(),
            listen: "0.0.0.0:8080".to_string(),
            skin_domains: Vec::new(),
        }
    }
}

/// [data] 数据目录配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DataConfig {
    /// 数据目录(数据库、密钥、材质),相对工作目录
    /// 环境变量 YGGR_DATA_DIR 可覆盖此值
    pub dir: PathBuf,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("data"),
        }
    }
}

/// [auth] 认证配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// 角色 UUID 生成策略
    pub player_uuid_generation: UuidGeneration,
    /// 令牌有效期(天)
    pub token_ttl_days: i64,
    /// 令牌有效窗口(天);超过此窗口但在 token_ttl_days 内的令牌为暂时失效状态
    pub token_active_window_days: i64,
    /// 是否允许非邮箱登录(角色名登录)
    pub non_email_login: bool,
    /// 登录/signout 每 IP 每分钟限流次数(0 表示不限)
    pub login_rate_limit_per_minute: u32,
    /// hasJoined 是否校验客户端 IP
    pub check_ip: bool,
    /// 每用户角色数量上限
    pub max_players_per_user: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            player_uuid_generation: UuidGeneration::Offline,
            token_ttl_days: 15,
            token_active_window_days: 15,
            non_email_login: true,
            login_rate_limit_per_minute: 10,
            check_ip: false,
            max_players_per_user: 5,
        }
    }
}

/// [user] 用户配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    /// 用户配置文件,相对路径基于配置目录(config_dir)
    pub file: Option<PathBuf>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            file: Some(PathBuf::from("user.toml")),
        }
    }
}

/// [meta] 实现信息
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MetaConfig {
    /// implementationName
    pub implementation_name: String,
    /// implementationVersion
    pub implementation_version: String,
    /// 注册页面地址(meta.links.register),None 则不输出
    pub register_url: Option<String>,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            implementation_name: "yggr".to_string(),
            implementation_version: env!("CARGO_PKG_VERSION").to_string(),
            register_url: None,
        }
    }
}

/// [features] 高级功能选项
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FeaturesConfig {
    /// legacy_skin_api:旧式皮肤 API 由服务端处理
    pub legacy_skin_api: bool,
    /// no_mojang_namespace:禁用 @mojang 命名空间
    pub no_mojang_namespace: bool,
    /// enable_mojang_anti_features:开启 Minecraft anti-features
    pub enable_mojang_anti_features: bool,
    /// username_check:启用用户名验证
    pub username_check: bool,
}

/// [frontend] 前端配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FrontendConfig {
    /// 前端静态文件目录(None 则不提供前端)
    /// 环境变量 YGGR_FRONTEND_DIR 可覆盖此值
    pub dir: Option<PathBuf>,
}

impl Default for FrontendConfig {
    fn default() -> Self {
        Self {
            dir: Some(PathBuf::from("frontend/dist")),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub data: DataConfig,
    pub auth: AuthConfig,
    pub user: UserConfig,
    pub meta: MetaConfig,
    pub features: FeaturesConfig,
    pub frontend: FrontendConfig,
}

impl Config {
    /// 从文件加载配置
    pub fn load(path: &Path) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    /// 应用环境变量覆盖与路径解析
    ///
    /// - `YGGR_DATA_DIR` 覆盖 data.dir
    /// - `YGGR_FRONTEND_DIR` 覆盖 frontend.dir
    /// - `user.file` 相对路径基于 config_dir 解析
    pub fn resolve_paths(&mut self, config_dir: &Path) {
        if let Ok(data_dir) = std::env::var("YGGR_DATA_DIR") {
            self.data.dir = PathBuf::from(data_dir);
        }
        if let Ok(frontend_dir) = std::env::var("YGGR_FRONTEND_DIR") {
            self.frontend.dir = Some(PathBuf::from(frontend_dir));
        }
        if let Some(ref user_file) = self.user.file
            && user_file.is_relative()
        {
            self.user.file = Some(config_dir.join(user_file));
        }
    }

    /// 从 base_url 推导材质域名白名单
    pub fn texture_domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = Vec::new();
        if let Ok(url) = url::Url::parse(&self.server.base_url)
            && let Some(host) = url.host_str()
        {
            domains.push(format!(".{}", host));
        }
        for d in &self.server.skin_domains {
            if !domains.contains(d) {
                domains.push(d.clone());
            }
        }
        domains
    }

    /// 暂时失效是否启用(token_active_window_days < token_ttl_days 时启用)
    pub fn temp_invalidation_enabled(&self) -> bool {
        self.auth.token_active_window_days < self.auth.token_ttl_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_domains() {
        let cfg = Config {
            server: ServerConfig {
                base_url: "https://ygg.example.com:8443/path".to_string(),
                skin_domains: vec!["example.org".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let domains = cfg.texture_domains();
        assert!(domains.contains(&".ygg.example.com".to_string()));
        assert!(domains.contains(&"example.org".to_string()));
    }

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.auth.player_uuid_generation, UuidGeneration::Offline);
        assert_eq!(cfg.auth.token_ttl_days, 15);
        assert!(!cfg.features.legacy_skin_api);
        assert!(!cfg.temp_invalidation_enabled());
    }
}
