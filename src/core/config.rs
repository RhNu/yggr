//! 服务配置:解析 config.toml

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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 服务器名称,显示在 meta.serverName
    pub server_name: String,
    /// 外部访问基础 URL,用于生成材质 URL,如 https://ygg.example.com
    pub base_url: String,
    /// 监听地址,如 0.0.0.0:8080
    pub listen: String,
    /// 数据目录(数据库、密钥、材质)
    pub data_dir: PathBuf,
    /// 角色 UUID 生成策略
    pub player_uuid_generation: UuidGeneration,
    /// 令牌有效期(天)
    pub token_ttl_days: i64,
    /// 是否允许非邮箱登录(角色名登录)
    pub non_email_login: bool,
    /// 额外的材质域名白名单规则(默认包含 base_url 的域名)
    pub skin_domains: Vec<String>,
    /// hasJoined 是否校验客户端 IP
    pub check_ip: bool,
    /// 登录/signout 每 IP 每分钟限流次数(0 表示不限)
    pub login_rate_limit_per_minute: u32,
    /// 是否应用种子配置 seed.toml
    pub seed_file: Option<PathBuf>,
    /// implementationName,显示在 meta.implementationName
    pub implementation_name: String,
    /// implementationVersion,显示在 meta.implementationVersion
    pub implementation_version: String,
    /// feature.legacy_skin_api:旧式皮肤 API 由服务端处理
    pub legacy_skin_api: bool,
    /// feature.no_mojang_namespace:禁用 @mojang 命名空间
    pub no_mojang_namespace: bool,
    /// feature.enable_mojang_anti_features:开启 Minecraft anti-features
    pub enable_mojang_anti_features: bool,
    /// feature.username_check:启用用户名验证
    pub username_check: bool,
    /// 注册页面地址(meta.links.register),None 则不输出
    pub register_url: Option<String>,
    /// 令牌有效窗口(天);超过此窗口但在 token_ttl_days 内的令牌为暂时失效状态
    /// 默认等于 token_ttl_days,即不启用暂时失效
    pub token_active_window_days: i64,
    /// 每用户角色数量上限
    pub max_players_per_user: u32,
    /// 前端静态文件目录(None 则不提供前端)
    pub frontend_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server_name: "My Yggdrasil".to_string(),
            base_url: "http://127.0.0.1:8080".to_string(),
            listen: "0.0.0.0:8080".to_string(),
            data_dir: PathBuf::from("data"),
            player_uuid_generation: UuidGeneration::Offline,
            token_ttl_days: 15,
            non_email_login: true,
            skin_domains: Vec::new(),
            check_ip: false,
            login_rate_limit_per_minute: 10,
            seed_file: Some(PathBuf::from("seed.toml")),
            implementation_name: "yggr".to_string(),
            implementation_version: env!("CARGO_PKG_VERSION").to_string(),
            legacy_skin_api: false,
            no_mojang_namespace: false,
            enable_mojang_anti_features: false,
            username_check: false,
            register_url: None,
            token_active_window_days: 15,
            max_players_per_user: 5,
            frontend_dir: Some(PathBuf::from("frontend/dist")),
        }
    }
}

impl Config {
    /// 从文件加载配置;文件不存在时使用默认值
    pub fn load(path: &Path) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    /// 从 base_url 推导材质域名白名单
    pub fn texture_domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = Vec::new();
        if let Ok(url) = url::Url::parse(&self.base_url)
            && let Some(host) = url.host_str()
        {
            domains.push(format!(".{}", host));
        }
        for d in &self.skin_domains {
            if !domains.contains(d) {
                domains.push(d.clone());
            }
        }
        domains
    }

    /// 暂时失效是否启用(token_active_window_days < token_ttl_days 时启用)
    pub fn temp_invalidation_enabled(&self) -> bool {
        self.token_active_window_days < self.token_ttl_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_domains() {
        let cfg = Config {
            base_url: "https://ygg.example.com:8443/path".to_string(),
            skin_domains: vec!["example.org".to_string()],
            ..Default::default()
        };
        let domains = cfg.texture_domains();
        assert!(domains.contains(&".ygg.example.com".to_string()));
        assert!(domains.contains(&"example.org".to_string()));
    }

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.player_uuid_generation, UuidGeneration::Offline);
        assert_eq!(cfg.token_ttl_days, 15);
        assert!(!cfg.legacy_skin_api);
        assert!(!cfg.temp_invalidation_enabled());
    }
}
