---
title: Onia Google Sign-In 接入
date: 2026-11-18
tags: onia, google, oauth, auth, android, flutter
---

# Onia Google Sign-In 接入

日期：2026-11-18。状态：控制台已配置完成（正式版），代码已实现，本地与生产服务端均已打通（bogus token 实测 401），生产已部署；dev APK 已装到真机，等用户解锁后点 Google 登录验证。代码未 commit。

## 控制台（Google Auth Platform）— 已完成

项目：`onia-prod-515106`（账号 richardzhang1999@163.com）。

- 品牌塑造：应用名称 Onia；应用主页 https://onia.asia；隐私政策 https://onia.asia/privacy-policy；服务条款 https://onia.asia/terms-of-service；已获授权的网域 onia.asia；已保存。
- 发布状态：正式版（Production）。测试用户曾加 richardzhang1999@163.com，转正式后测试用户不再限制登录。
- OAuth 客户端：
  - `Onia Android`（已有）：包名 `com.colna.onia.onia_client`，SHA-1 `9B:F2:99:51:B5:0E:18:81:FE:D5:12:20:14:78:D7:ED:BD:D2:C1:EF`，client ID `328960377279-uvct4jlkb5c5ahhi6e83i6ioo9e24k0bp.apps.googleusercontent.com`。
  - `Onia Android Dev`（新建）：包名 `com.colna.onia.onia_client.dev`，同一 SHA-1，client ID `328960377279-mj6f9hpr8j7e65buptl9ms7f81e9ddad.apps.googleusercontent.com`。
  - `Onia Web`（已有，作 serverClientId）：client ID `328960377279-salium6oef7gll6qkibqm62n99qcibcc.apps.googleusercontent.com`；JS 来源 onia.asia / admin.onia.asia / localhost:5173；重定向 URI 含 api.onia.asia/v1/auth/google/callback（当前 Android ID token 流程不用）。
- 旧 release 包与 dev 包目前都用 debug keystore 签名，所以两个 Android client 可共用同一 SHA-1；换正式签名后要补新指纹。

## 配置

- `apps/client/config/dev.json`、`config/prod.json` 已加 `ONIA_GOOGLE_SERVER_CLIENT_ID=<Onia Web client ID>`。
- 本地 `.env` 已加 `ONIA_GOOGLE_CLIENT_IDS=<Onia Web client ID>`。
- 服务端 `ONIA_GOOGLE_CLIENT_IDS` 为 `aud` 白名单；空则 `/v1/auth/google` 返回 422「not configured」。

## 代码实现

- 协议：新增 kind `auth.google` / `auth.google.ok` 与 `AuthGooglePayload { id_token, locale }`（crates/onia-protocol）。
- 服务端：
  - `/v1/auth/google`（crates/onia-server/src/http/auth.rs）。
  - `GoogleVerifier`（crates/onia-server/src/google.rs）：JWKS（`https://www.googleapis.com/oauth2/v3/certs`，1h 缓存、未知 kid 刷新一次）、RS256、iss（两种写法）、aud、exp、email_verified。
  - 用户绑定：先按 `google_sub` 找，再按 email 找并补绑；都没有则新建（password_hash 用哨兵 `!google-sign-in-only`，`email_verified_at=now`）。
  - 迁移 `0014_google_identity.sql`：`users.google_sub` + partial unique index。
- 客户端：
  - 依赖 `google_sign_in 7.2.0`；登录页「使用 Google 继续」+ 或分割线。
  - `runtime/google_auth.dart`：`initialize(serverClientId:)` + `authenticate()`，取消返回 null。
  - `session.loginWithGoogle` → `auth.google`。

## 验证

- 官网：build + rsync 到 `/opt/onia/web`；headless Chrome 确认 `/privacy-policy`、`/terms-of-service` 渲染。
- 服务端：`cargo test -p onia-server` 61 passed（google 模块 8 个）。
- 本地服务端：重启新二进制后 POST `/v1/auth/google` + 假 token 返回 401 `invalid google token`（路由与配置生效）。
- 客户端：`flutter analyze` 干净；`flutter test` 113 passed；dev debug APK 已用带 client ID 的 `config/dev.json` 重新构建。
- 本地 dev API：`http://192.168.3.24:8080`；手机 serial `f7031061` 在线。

## 生产部署（2026-11-18 17:56）

- 本地 `docker buildx --platform linux/amd64 --pull=false` 构建 `onia-server:latest`（image id `7f1a00e7866d`，37.7MB gz），临时 context 只含 Cargo.toml/Cargo.lock/crates/apps/server + 临时 Dockerfile（rust:1.85-bookworm → debian:12-slim）。
- SSH 到 47.250.210.196 间歇性 KEX 超时；用 macOS 自带 rsync 的 `--partial --append` 重试 4 次传完，sha256 一致；用 `aliyun swas-open RunCommand` 校验 + `docker load` + 改 `/opt/onia/.env` + `docker compose up -d --force-recreate onia-server`。
- 迁移 `0014` success；`users.google_sub` 存在；`https://api.onia.asia/v1/auth/google` 假 token 返回 401 `invalid google token`；`/health` ok。
- 坑：macOS 自带 rsync 2.6.9 不支持 `--append-verify`，用 `--append`。

## 踩坑

- `.env` 的 `ONIA_EMAIL_FROM=Onia <no-reply@onia.asia>` 未加引号，`dotenvy::from_path` 解析失败（main.rs 忽略了错误），导致该行之后的变量（含 `ONIA_GOOGLE_CLIENT_IDS`）都不加载。已改为带引号。详见 [dotenvy-unquoted-value-stops-loading.md](../../30-Resources/troubleshooting/dotenvy-unquoted-value-stops-loading.md)。
- Chrome 控制台页面用无障碍树操作时，保存后 webarea 会从树里消失、窗口 inventory 也会不稳定；改用 Chrome AppleScript `execute javascript` 直接操作 DOM 才稳定完成创建 client。

## 待办

- 真机：dev APK 已安装并启动（serial `f7031061`，Wi-Fi `presence-5g` 192.168.3.146，同网段可达 192.168.3.24:8080）；等用户解锁手机后点「使用 Google 继续」，确认能建号/登录并进入工作台。
- 代码未 commit；iOS 需要单独的 iOS OAuth client + URL scheme。
- 备注：dev 包指向本地 `http://192.168.3.24:8080`；prod 包指向 `https://api.onia.asia`。生产环境未配置 chat agent（mock），与本次无关。
