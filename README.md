# YggR - 轻量 Yggdrasil 认证服务端

一个用 Rust 实现的轻量级 [Yggdrasil](https://yushijinhun.github.io/authlib-injector/zh/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83.html) 认证服务端,兼容 [authlib-injector](https://yushijinhun.github.io/authlib-injector/zh/Home.html) 规范。

## 快速开始

```bash
cargo build --release

# 配置与用户(参考示例文件)
cp config/config.example.toml config/config.toml
cp config/user.example.toml config/user.toml
# 编辑 config/user.toml 设置你的邮箱/用户名与强密码

# 启动
./target/release/yggr
```

默认目录结构:配置文件在 `config/`,数据文件在 `data/`。
可通过环境变量覆盖:

| 环境变量            | 默认            | 说明                             |
| ------------------- | --------------- | -------------------------------- |
| `YGGR_CONFIG_DIR`   | `config`        | 配置目录(config.toml, user.toml) |
| `YGGR_DATA_DIR`     | `data`          | 数据目录(数据库、密钥、材质)     |
| `YGGR_FRONTEND_DIR` | `frontend/dist` | 前端静态文件目录                 |

启动时自动完成:生成 RSA 密钥(`data/private_key.pem`)-> 初始化 SQLite(`data/yggr.db`)-> 初始化用户。

## 文档

- [`docs/architecture.md`](docs/architecture.md) - 服务端架构文档(协议约定、数据模型、API 全景、安全设计)
- [`docs/spec-compliance.md`](docs/spec-compliance.md) - authlib-injector 规范符合性核对表

## 测试

```bash
cargo test        # 单元 + 集成测试
```
