# authlib-injector 规范符合性核对表

> 对照《[Yggdrasil 服务端技术规范](https://yushijinhun.github.io/authlib-injector/zh/Yggdrasil-%E6%9C%8D%E5%8A%A1%E7%AB%AF%E6%8A%80%E6%9C%AF%E8%A7%84%E8%8C%83.html)》逐条核对 yggr 现有实现。
> 架构总览见 [`architecture.md`](architecture.md)。

---

## 1. 基本约定

| #    | 规范要求                                                                                                            | 实现位置                                   | 说明                                                                                                          |
| ---- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| 1.1  | UTF-8 编码                                                                                                          | 全局                                       |                                                                                                               |
| 1.2  | JSON 请求/响应                                                                                                      | 全部端点                                   |                                                                                                               |
| 1.3  | `Content-Type: application/json; charset=utf-8`                                                                     | `core/types.rs::JsonResponse`              | 错误与成功响应统一输出,均带 charset                                                                           |
| 1.4  | 所有 API 使用 HTTPS                                                                                                 | 部署层                                     | yggr 自身 HTTP,生产须反代终结 TLS(README 已说明)                                                            |
| 1.5  | 错误格式 `{error, errorMessage, cause}`                                                                             | `core/error.rs::ApiError`                  | cause 恒为 null(序列化输出),规范允许省略                                                                      |
| 1.6  | 令牌无效 -> `403 ForbiddenOperationException / Invalid token.`                                                       | `core/error.rs::invalid_token`             | validate/refresh/join 共用                                                                                    |
| 1.7  | 密码错误或短时多次登录失败 -> `403 ForbiddenOperationException / Invalid credentials. Invalid username or password.` | `core/error.rs::invalid_credentials`       | authenticate/signout;限流超限同样返回此错误(符合"按密码错误处理")                                             |
| 1.8  | 向已绑定角色令牌指定角色 -> `400 IllegalArgumentException / Access token already has a profile assigned.`            | `core/error.rs::profile_already_assigned`  | refresh 中 `selectedProfile` + 已绑定角色                                                                     |
| 1.9  | 向令牌绑定不属于其用户的角色(非标准)                                                                                | `core/error.rs::invalid_profile_selection` | 规范此项为非标准,要求 403 ForbiddenOperationException;yggr 返回 403,errorMessage 为 `Profile is not owned by the user.` |
| 1.10 | 使用错误角色加入服务器 -> `403 ForbiddenOperationException / Invalid token.`                                         | `api/session.rs::join`                     | selectedProfile 与令牌绑定角色不一致                                                                          |
| 1.11 | 无符号 UUID 数据格式                                                                                                | 全局                                       | id/URL 路径均无连字符                                                                                         |

## 2. 模型

### 2.1 用户

| #     | 规范要求                             | 实现位置                      | 说明                   |
| ----- | ------------------------------------ | ----------------------------- | ---------------------- |
| 2.1.1 | 用户 ID(无符号 UUID)、邮箱唯一、密码 | `core/db`(users 表)           |                        |
| 2.1.2 | 用户序列化 `{id, properties}`        | `core/types.rs::UserResponse` | 含 `preferredLanguage` |

### 2.2 角色

| #      | 规范要求                                                                          | 实现位置                                         | 说明                                                                                   |
| ------ | --------------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------- |
| 2.2.1  | UUID 全局唯一、名称全局唯一但可变                                                 | `core/db`(players 表)                            | name UNIQUE                                                                            |
| 2.2.2  | 随机 UUID v4 或离线兼容 `UUID.nameUUIDFromBytes("OfflinePlayer:"+name)`           | `core/crypto.rs::offline_uuid` / `random_uuid`   | 有交叉验证单测                                                                         |
| 2.2.3  | 角色序列化 `{id, name, properties?}`                                              | `core/types.rs::ProfileResponse`                 |                                                                                        |
| 2.2.4  | `textures` 属性 = Base64(JSON: timestamp/profileId/profileName/textures)          | `app/textures::build_textures_value`             | timestamp 为 Java 毫秒                                                                 |
| 2.2.5  | `signature` = SHA1withRSA Base64                                                  | `core/crypto.rs::sign_sha1`                      | 仅 `unsigned=false` / hasJoined 时输出                                                 |
| 2.2.6  | 材质 metadata.model 取 `default`/`slim`                                           | `app/textures`                                   | 仅 slim 输出 metadata;slim 存库为 `slim`,default 存 `classic`                          |
| 2.2.7  | `uploadableTextures` 属性(逗号分隔,如 `skin`、`skin,cape`);不存在则该角色不可上传 | `api/session.rs::build_profile_response`         | 含 properties 的响应(hasJoined、profile 查询)中输出 `skin,cape`(yggr 两种材质均可上传) |
| 2.2.8  | 材质 URL:文件名 = hash,hash 由服务端计算                                          | `app/textures::TextureStore`                     | SHA-256 内容寻址                                                                       |
| 2.2.9  | 材质响应 `Content-Type: image/png`                                                | `api/profiles.rs::texture_file`                  | 防 MIME Sniffing                                                                       |
| 2.2.10 | 上传材质安全:先读尺寸(防 PNG bomb)、校验合法尺寸、重编码去元数据                  | `app/textures::sanitize_png`                     | 仅读头部取尺寸;>1024 拒绝;统一 RGBA8 重编码                                            |
| 2.2.11 | 皮肤尺寸:64x32 或 64x64 整数倍                                                    | `app/textures::validate_dimensions`              |                                                                                        |
| 2.2.12 | 披风尺寸:64x32 或 22x17 整数倍;22x17 补足到 64x32 倍数                            | `app/textures::validate_dimensions` / `pad_cape` |                                                                                        |

### 2.3 令牌

| #     | 规范要求                             | 实现位置                       | 说明                                                                                                                                               |
| ----- | ------------------------------------ | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.3.1 | accessToken 服务端随机生成、可作主键 | `core/crypto.rs::random_token` | 32 字节随机 hex                                                                                                                                    |
| 2.3.2 | clientToken 客户端提供、无唯一性     | `core/db`(tokens 表)           |                                                                                                                                                    |
| 2.3.3 | 绑定角色可为空                       | `core/db`(tokens.player_id)    |                                                                                                                                                    |
| 2.3.4 | 颁发时间、过期时限(如 15 天)         | `core/db`(tokens 表)           | `token_ttl_days` 默认 15                                                                                                                           |
| 2.3.5 | 状态不可逆;刷新仅颁新令牌            | `api/auth.rs::refresh`         | 旧令牌删除                                                                                                                                         |
| 2.3.6 | 暂时失效状态(角色改名触发/时间阈值)    | `api/auth.rs::TokenStatus`     | 通过 `token_active_window_days` 配置(默认 = token_ttl_days,即不启用);小于 ttl 时启用:令牌超过 active window 但未过期为暂时失效,仅可刷新            |
| 2.3.7 | 令牌数量上限(如 10 个),超限吊销最旧  | `core/db::enforce_token_limit` | 每用户上限 10(`api/auth.rs::MAX_TOKENS_PER_USER`),颁发后按 issued_at 吊销最旧;过期令牌由启动时与后台任务定期清理(`core/db::delete_expired_tokens`) |
| 2.3.8 | refresh 失败时原令牌依然有效         | `api/auth.rs::refresh`         | 先颁发新令牌,成功后再吊销原令牌;失败时原令牌保持有效                                                                                               |

## 3. 认证 API(/service/authserver)

| #    | 端点         | 规范要求                                                                | 实现位置                    | 说明                                            |
| ---- | ------------ | ----------------------------------------------------------------------- | --------------------------- | ----------------------------------------------- |
| 3.1  | authenticate | clientToken 缺省生成随机 UUID;任何 clientToken 可接受                   | `api/auth`                  |                                                 |
| 3.2  | authenticate | 角色绑定:无角色->空、单角色->自动绑定、多角色->空                          | `api/auth.rs::authenticate` |                                                 |
| 3.3  | authenticate | `requestUser` 缺省 false                                                | `api/auth`                  |                                                 |
| 3.4  | authenticate | 响应含 accessToken/clientToken/availableProfiles/selectedProfile?/user? | `core/types`                | availableProfiles 恒输出(可为空数组)            |
| 3.5  | authenticate | 限流"应针对用户,而不是客户端 IP"                                        | `app/state.rs::RateLimiter` | 按 IP(`X-Forwarded-For` 首值)与用户名双维度限流 |
| 3.6  | authenticate | 角色名登录:`feature.non_email_login=true` + 自动绑定登录所用角色        | `api/meta` / `api/auth`     |                                                 |
| 3.7  | refresh      | clientToken 提供则校验,否则只查 accessToken                             | `api/auth.rs::refresh`      |                                                 |
| 3.8  | refresh      | 新令牌 clientToken 与原令牌相同                                         | `api/auth.rs::refresh`      |                                                 |
| 3.9  | refresh      | selectedProfile 角色选择:仅原令牌未绑定角色时可执行,须属于该用户        | `api/auth.rs::refresh`      |                                                 |
| 3.10 | refresh      | 暂时失效令牌仍可刷新                                                    | `api/auth.rs::refresh`      | 接受 `TemporarilyInvalid` 状态令牌               |
| 3.11 | validate     | 有效 -> 204,否则按令牌无效处理                                           | `api/auth.rs::validate`     | 暂时失效令牌视为无效                             |
| 3.12 | invalidate   | 只检查 accessToken;无论成败返回 204                                     | `api/auth.rs::invalidate`   |                                                 |
| 3.13 | signout      | 验密后吊销用户全部令牌 -> 204                                            | `api/auth.rs::signout`      |                                                 |
| 3.14 | signout      | 与登录同等限流                                                          | `api/auth.rs::signout`      | IP + 用户名双维度,与 authenticate 一致          |

## 4. 会话 API(/service/sessionserver)

| #   | 端点           | 规范要求                                                  | 实现位置                     | 说明                                                                                       |
| --- | -------------- | --------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------ |
| 4.1 | join           | 令牌有效且 selectedProfile 与绑定角色一致才成功           | `api/session.rs::join`       | 失败返回 403 Invalid token.                                                                |
| 4.2 | join           | 记录 serverId/accessToken/客户端 IP;内存存储,过期(如 30s) | `api/session.rs::join`       | 记录 player_id/name/ip(以角色代替 accessToken,功能等价);`JOIN_TTL_MS=30_000`;后台 60s 清理 |
| 4.3 | join           | serverId 作主键;成功 -> 204                                | `api/session.rs::join`       |                                                                                            |
| 4.4 | hasJoined      | username 须与记录角色名一致                               | `api/session.rs::has_joined` |                                                                                            |
| 4.5 | hasJoined      | ip 参数可选,`prevent-proxy-connections` 开启时校验        | `api/session.rs::has_joined` | 对应 `check_ip` 配置;缺 ip 参数时放行                                                      |
| 4.6 | hasJoined      | 成功 -> 角色完整信息(含签名);失败 -> 204                    | `api/session.rs::has_joined` | 含签名 textures(`signed=true`)                                                             |
| 4.7 | hasJoined      | -(防重放)                                                 | `api/session.rs::has_joined` | 超越规范的安全加强:一次性消费                                                              |
| 4.8 | profile/{uuid} | `unsigned` 缺省 true(无签名);`false` 含签名               | `api/session.rs::profile`    | 仅 `unsigned=false` 时签名                                                                 |
| 4.9 | profile/{uuid} | 角色不存在 -> 204                                          | `api/session.rs::profile`    |                                                                                            |

## 5. 角色 API(/service/api)

| #   | 端点                    | 规范要求                                              | 实现位置                          | 说明                                             |
| --- | ----------------------- | ----------------------------------------------------- | --------------------------------- | ------------------------------------------------ |
| 5.1 | /service/api/profiles/minecraft | 返回命中角色,不含 properties,不存在的跳过,次序无要求  | `api/profiles.rs::batch_profiles` | 保持输入顺序                                     |
| 5.2 | /service/api/profiles/minecraft | 单次查询上限(至少 2,防 CC)                            | `api/profiles`                    | 上限 100,超限 400                                |
| 5.3 | 材质上传                | `Authorization: Bearer {accessToken}`;缺失/无效 -> 401 | `api/profiles.rs::require_bearer` | 中间件在路由层执行,未认证请求返回 401 而非 400   |
| 5.4 | 材质上传                | 成功 -> 204                                            | `api/profiles`                    |                                                  |
| 5.5 | 材质上传                | PUT multipart:model(仅皮肤,slim/空)+ file(image/png)  | `api/profiles.rs::upload_texture` | file 未带 Content-Type 时放行;带则必须 image/png |
| 5.6 | 材质上传                | DELETE 清除 -> 恢复默认                                | `api/profiles.rs::delete_texture` |                                                  |

## 6. 扩展 API

| #    | 规范要求                                                                    | 实现位置                           | 说明                                                                                                         |
| ---- | --------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 6.1  | `GET /service` 返回 meta + skinDomains + signaturePublickey                   | `api/meta.rs`                      |                                                                                                              |
| 6.2  | signaturePublickey PEM 格式(仅允许换行空白)                                 | `core/crypto.rs::public_key_pem`   | LineEnding::LF                                                                                               |
| 6.3  | skinDomains 规则:`.` 前缀匹配子域,否则精确匹配                              | `core/config.rs::texture_domains`  | 默认推 `.{base_url host}`                                                                                    |
| 6.4  | meta:serverName/implementationName/implementationVersion/links              | `api/meta.rs`                      | links.homepage = base_url;links.register = register_url(可选)                                              |
| 6.5  | `feature.non_email_login`                                                   | `api/meta.rs`                      | 跟随配置                                                                                                     |
| 6.6  | `feature.legacy_skin_api`(可选)                                             | `api/meta.rs` / `api/profiles.rs`  | 配置 `legacy_skin_api=true` 时输出;同时启用 `GET /service/skins/MinecraftSkins/{username}.png` 服务端处理    |
| 6.7  | `feature.no_mojang_namespace`(可选)                                         | `api/meta.rs`                      | 配置 `no_mojang_namespace=true` 时输出                                                                       |
| 6.8  | `feature.enable_mojang_anti_features`(可选)                                 | `api/meta.rs`                      | 配置 `enable_mojang_anti_features=true` 时输出                                                                |
| 6.9  | `feature.enable_profile_key`                                                | `api/meta` / `api/certificates.rs` | true,已实现 certificates                                                                                     |
| 6.10 | `feature.username_check`(可选)                                              | `api/meta.rs`                      | 配置 `username_check=true` 时输出                                                                             |
| 6.11 | ALI 头 `X-Authlib-Injector-API-Location`                                    | `api/meta.rs` / `api/mod.rs`       | 值 `/service`(指向自身);根路径 GET / 同样返回此头                                                              |
| 6.12 | 签名密钥:RSA(推荐 4096)、避免密钥变化、多实例共享                           | `core/crypto.rs::generate_keypair` | RSA-4096 主签名密钥(持久化于 `data/private_key.pem` 保证稳定);certificates 会话密钥保持 2048(与 Mojang 一致) |
| 6.13 | certificates:publicKeySignature = SHA1withRSA(publicKey DER)                | `api/certificates.rs`              |                                                                                                              |
| 6.14 | certificates:publicKeySignatureV2 = SHA256withRSA({expiresAt,keyPair} JSON) | `api/certificates.rs`              | 固定紧凑 JSON 格式,与 Gson 输出一致                                                                          |
| 6.15 | certificates:expiresAt/refreshedAfter RFC3339、Bearer 认证                  | `api/certificates.rs`              | 7 天有效期 / 1 天建议刷新;要求令牌已绑定角色                                                                 |

## 7. 汇总

### 全部规范项已符合

- 暂时失效令牌状态(§2.3.6)已实现:通过 `token_active_window_days` 配置时间阈值
- `legacy_skin_api` feature 选项已实现:配置为 true 时服务端处理旧式皮肤 API
- `no_mojang_namespace` / `enable_mojang_anti_features` / `username_check` feature 选项已实现:配置驱动,默认 false
- `links.register` 已实现:配置 `register_url` 时输出
