---
title: Onia 脚本 OTA 与热更新
date: 2026-11-17
tags: project, onia, social-proxy, scripts, ota, hot-update
---

# Onia 脚本 OTA 与热更新

## 目标

脚本不再只维护在客户端 assets：服务端保存版本、客户端下载并校验 SHA-256、缓存后使用；收到更新时无需重装 App，页面刷新即可生效；内置 assets 只作为首次启动和离线回退。

## 服务端

迁移 `0011_script_releases.sql` 扩展 `script_releases`：

- `sha256`：脚本 SHA-256
- `content`：完整 JS bundle（TEXT）
- `min_app_version`
- `notes`
- `active`
- `released_at`

接口：

- `GET /v1/devices/scripts/current`：设备 token 鉴权，返回当前 active release 元数据。
- `GET /v1/devices/scripts/download/{version}`：返回 `application/javascript`，`no-store` + ETag。
- `POST /v1/admin/scripts/create`：envelope `script_release.create`，上传版本并计算 sha256，可立即激活。
- `POST /v1/admin/scripts/list`：envelope `script_release.list`。
- `POST /v1/admin/scripts/activate`：envelope `script_release.activate`，用于启用/回滚。

推送：

- 客户端 `device.hello.runtime` 上报 `script_version`、`script_sha256`、`app_version`。
- 服务端在 `device.welcome` 后比较 active release 与客户端版本；不同则推 `script.update { version, sha256, size, url, mandatory, released_at }`。
- `min_app_version` 不满足时跳过推送。
- 版本可降级：后台激活旧版本即回滚。

## 客户端

- `apps/client/lib/runtime/script_repository.dart`：
  - 优先读 `SessionStore` 里缓存的 `script_source/version/sha256`，校验 SHA-256 后使用。
  - 无缓存或校验失败时回退 `rootBundle` 的 `social_proxy.js`。
  - `save()` 只有 SHA-256 匹配才落盘；保留当前版本。
- `SessionStore` 增加脚本缓存字段；实现于 `PrefsSessionStore`，测试 fake 同步补齐。
- `OniaApi` 增加 `getScriptCurrent()` 和 `downloadScript(version)`。
- `DeviceRuntime`：
  - `device.welcome` 后主动拉一次 current（兜底服务端未推）。
  - 处理 `script.update`：下载、校验、调用 `InstagramSession.applyScriptRelease`。
  - 有 Behavior 在跑时不立即刷新页面，只缓存；空闲时 `controller.reload()`，页面刷新生效。
  - `device.hello` runtime 上报脚本版本/哈希。
- `InstagramSession`：
  - `start()` 从 `ScriptRepository.load()` 取 bundle。
  - `applyScriptRelease` 保存并切换内存 bundle；登录且空闲时 reload。
- `crypto` 加入 pubspec 直接依赖。

## 管理端

新增 `/scripts` 页面：

- 上传 JS 文件或粘贴内容，填写版本、最低 App 版本、说明、是否立即启用。
- 版本列表展示 active、SHA-256、大小、发布时间、说明。
- 支持一键启用/回滚。

## 验证

- 真机 `get_profile`（NASA）：12 条帖子、1 条当前 Story、6 组 highlights（100 个 items），无错误；分页 cursor 正常返回。
- 本地服务端：create/list/current/download 均通过；假设备 WS hello 收到 `script.update`。
- `cargo test --workspace` 通过；scripts 133 tests；Flutter 92 tests；admin/web/dev-device typecheck 通过。
- 本地 Postgres 已执行 0011，`script_releases` 列已存在。

## 限制与后续

- 真机 OTA 热更新没有在现有生产 App 上实测：需要把 App 指向本地 API 并重新登录，或部署服务端到生产后再测；客户端下载/应用逻辑和 WS 推送已分别单测/本地验证。
- 当前只校验 SHA-256 + TLS + 设备 token，未做 Ed25519 签名；如需防服务端被篡改，再加签名。
- `gray` 字段尚未使用；后续可做按设备/用户灰度。
