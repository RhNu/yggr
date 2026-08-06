//! 应用共享状态:配置、数据库、密钥、材质存储、会话缓存、限流

use rsa::{RsaPrivateKey, RsaPublicKey};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

use crate::config::Config;
use crate::textures::TextureStore;

/// hasJoined 会话记录(join 后短暂有效)
#[derive(Debug, Clone)]
pub struct JoinRecord {
    pub player_id: String,
    pub player_name: String,
    pub ip: Option<String>,
    pub expires_at: i64,
}

/// 简单固定窗口限流(按 IP),用于登录/signout
pub struct RateLimiter {
    limit: u32,
    window_secs: i64,
    inner: Mutex<HashMap<IpAddr, (i64, u32)>>,
}

impl RateLimiter {
    pub fn new(limit: u32) -> Self {
        RateLimiter {
            limit,
            window_secs: 60,
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 检查并计数;返回 false 表示已超限
    pub fn check(&self, ip: IpAddr) -> bool {
        if self.limit == 0 {
            return true;
        }
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let mut map = self.inner.lock().unwrap();
        // 清理过期条目,防止无限增长
        if map.len() > 10_000 {
            map.retain(|_, (window, _)| *window > now - self.window_secs * 2);
        }
        let entry = map.entry(ip).or_insert((now, 0));
        if entry.0 != now {
            *entry = (now, 0);
        }
        if entry.1 >= self.limit {
            return false;
        }
        entry.1 += 1;
        true
    }
}

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: SqlitePool,
    pub store: TextureStore,
    pub private_key: Arc<RsaPrivateKey>,
    pub public_key: RsaPublicKey,
    /// serverId -> join 记录(内存,30 秒过期)
    pub sessions: Arc<Mutex<HashMap<String, JoinRecord>>>,
    /// 登录/signout 限流
    pub limiter: Arc<RateLimiter>,
}

impl AppState {
    /// 清理过期的 join 会话记录
    pub fn cleanup_sessions(&self) {
        let now = OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let mut map = self.sessions.lock().unwrap();
        map.retain(|_, record| record.expires_at > now);
    }
}
