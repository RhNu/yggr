# yggr - 自用 Yggdrasil 认证服务端

一个用 Rust 实现的轻量级 [Yggdrasil](https://yushijinhun.github.io/authlib-injector/zh/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83.html) 认证服务端,兼容 [authlib-injector](https://yushijinhun.github.io/authlib-injector/zh/Home.html) 规范,自用场景设计。

技术栈:Rust + [axum](https://github.com/tokio-rs/axum) + SQLite。单二进制,零外部依赖。

## 功能

- 认证 API:`authenticate` / `refresh` / `validate` / `invalidate` / `signout`(支持邮箱与角色名登录)
- 会话 API:`join` / `hasJoined`(内存会话,30 秒过期,一次性防重放)
- 角色 API:`profile/{uuid}`(含 SHA1withRSA 签名的 `textures` 属性)、`POST /service/api/profiles/minecraft` 批量查询
- 材质系统:皮肤/披风上传与清除,PNG 安全校验(防 PNG bomb、去元数据重编码、22x17 披风自动补足)、SHA-256 内容寻址存储
- 元数据:`GET /service` 返回 meta + skinDomains + signaturePublickey,含 `X-Authlib-Injector-API-Location` 头;根路径 `/` 返回 ALI 头
- Minecraft 1.19+ 消息签名密钥:`POST /service/minecraftservices/player/certificates`(V1/V2 签名)
- 离线兼容角色 UUID(`MD5("OfflinePlayer:"+name)`),可与离线服平滑迁移;也可随机或手动指定 UUID
- 登录限流(每 IP)、argon2id 密码哈希、RSA-4096 自动生成密钥
- 可选暂时失效令牌状态(`token_active_window_days`)
- 可选 feature 选项:`legacy_skin_api` / `no_mojang_namespace` / `enable_mojang_anti_features` / `username_check`

## 快速开始

```bash
cargo build --release

# 配置与种子用户(参考示例文件)
cp config/config.example.toml config/config.toml
cp config/seed.example.toml config/seed.toml
# 编辑 config/seed.toml 设置你的邮箱/用户名与强密码

# 启动
./target/release/yggr
```

默认目录结构:配置文件在 `config/`,数据文件在 `data/`。
可通过环境变量覆盖:

| 环境变量           | 默认     | 说明                          |
| ------------------ | -------- | ----------------------------- |
| `YGGR_CONFIG_DIR`  | `config` | 配置目录(config.toml, seed.toml) |
| `YGGR_DATA_DIR`    | `data`   | 数据目录(数据库、密钥、材质)    |
| `YGGR_FRONTEND_DIR`| `frontend/dist` | 前端静态文件目录              |

启动时自动完成:生成 RSA 密钥(`data/private_key.pem`)-> 初始化 SQLite(`data/yggr.db`)-> 应用种子用户。

## 客户端接入

### authlib-injector(推荐)

在启动器中填写的 Yggdrasil 地址即服务根地址,例如 `http://127.0.0.1:8080/`(生产环境务必置于 HTTPS 反向代理之后)。authlib-injector 会通过根路径的 ALI 头自动发现 `/service` 下的 API。

### 无 authlib-injector(自写客户端/服务端)

实现以下端点即可(见 `src/lib.rs`):

| 端点                                                                   | 说明                       |
| ---------------------------------------------------------------------- | -------------------------- |
| `GET /`                                                                | ALI 头(指向 /service)     |
| `GET /service`                                                         | 元数据 + 公钥              |
| `POST /service/authserver/authenticate`                                 | 登录                       |
| `POST /service/authserver/refresh`                                      | 刷新令牌                   |
| `POST /service/authserver/validate`                                     | 验证令牌(204)              |
| `POST /service/authserver/invalidate`                                   | 吊销令牌(204)              |
| `POST /service/authserver/signout`                                      | 登出(204)                  |
| `POST /service/sessionserver/session/minecraft/join`                    | 客户端进服                 |
| `GET /service/sessionserver/session/minecraft/hasJoined`                | 服务端验客户端             |
| `GET /service/sessionserver/session/minecraft/profile/{uuid}?unsigned=` | 角色属性(签名)             |
| `POST /service/api/profiles/minecraft`                                  | 批量查询(≤100)             |
| `PUT/DELETE /service/api/user/profile/{uuid}/{skin\|cape}`              | 材质上传/清除(Bearer)      |
| `GET /service/textures/{hash}`                                          | 材质文件(image/png)        |
| `POST /service/minecraftservices/player/certificates`                   | 1.19+ 消息签名密钥(Bearer) |

## 配置

见 [`config/config.example.toml`](config/config.example.toml),关键项:

- `base_url`:外部访问地址,用于生成材质 URL(反代场景填 https 域名)
- `player_uuid_generation`: `offline`(离线兼容)或 `random`;seed 中可逐角色指定 `uuid`
- `non_email_login`: 允许角色名登录(自动绑定该角色)
- `check_ip`: hasJoined 时校验 IP(反代需正确传 `X-Forwarded-For`)
- `login_rate_limit_per_minute`: 登录/登出每 IP 限流
- `token_active_window_days`: 令牌有效窗口;小于 `token_ttl_days` 时启用暂时失效
- `legacy_skin_api` / `no_mojang_namespace` / `enable_mojang_anti_features` / `username_check`: 可选 feature
- `register_url`: 注册页面地址(meta.links.register)

## 用户与角色管理

通过 [`config/seed.toml`](config/seed.example.toml) 在启动时创建用户、角色与初始皮肤/披风。已存在的用户/角色自动跳过;修改密码或新增角色后重启即可。

## 安全说明

- 生产部署必须使用 HTTPS(建议反向代理终结 TLS)
- `config/seed.toml` 含明文密码,请勿提交到版本库
- 材质上传已做 PNG 安全处理(尺寸上限 1024、重新编码去元数据),防止恶意材质

## 文档

- [`docs/architecture.md`](docs/architecture.md) - 服务端架构文档(协议约定、数据模型、API 全景、安全设计)
- [`docs/spec-compliance.md`](docs/spec-compliance.md) - authlib-injector 规范符合性核对表

## 测试

```bash
cargo test        # 单元 + 集成测试
```

集成测试覆盖完整链路:meta -> authenticate -> validate -> join -> hasJoined -> profile(验签)-> 材质上传/下载 -> refresh -> certificates -> invalidate -> signout。
