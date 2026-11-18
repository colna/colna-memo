---
title: Onia Google Sign-In 接入
date: 2026-11-18
tags: onia, google, oauth, auth, android, flutter
---

# Onia Google Sign-In 接入

日期：2026-11-18。状态：代码已实现（未提交 / 服务端未部署 / 未真机联调），控制台侧还差 Branding、转 Production、测试用户和调试包 Android client。

## 控制台（Google Auth Platform）待办

- Branding：
  - 应用主页：https://onia.asia/
  - 隐私政策：https://onia.asia/privacy-policy（已部署并实测渲染）
  - 服务条款：https://onia.asia/terms-of-service（同上）
  - 已获授权的网域：onia.asia
- 发布状态：Testing → Production。使用 email/profile/openid 基础 scope 无需审核；发布后任意 Google 用户可登录，不必逐个加测试用户。
- 测试用户（Testing 阶段用）：richardzhang1999@163.com
- OAuth 客户端：
  - Android（release，已有）：`com.colna.onia.onia_client`。
  - Android（debug，待建）：`com.colna.onia.onia_client.dev`；SHA-1 `9B:F2:99:51:B5:0E:18:81:FE:D5:12:20:14:78:D7:ED:BD:D2:C1:EF`（来自 `~/.android/debug.keystore`；当前 release 也用 debug 签名，两个 client 可共用同一 SHA-1）。
  - Web（必须有，作 serverClientId）：类型 Web application。Android Credential Manager 需要它才能拿到 ID token。拿到 client ID 后：填入 `apps/client/config/dev.json` / `prod.json` 的 `ONIA_GOOGLE_SERVER_CLIENT_ID`，并给服务端 `ONIA_GOOGLE_CLIENT_IDS`。

## 代码实现

- 协议：新增 kind `auth.google` / `auth.google.ok` 与 `AuthGooglePayload { id_token, locale }`（crates/onia-protocol）。
- 服务端：
  - `/v1/auth/google`（crates/onia-server/src/http/auth.rs）。
  - `GoogleVerifier`（crates/onia-server/src/google.rs）：JWKS（`https://www.googleapis.com/oauth2/v3/certs`，1h 缓存、未知 kid 刷新一次）、RS256、iss（两种写法）、aud（`ONIA_GOOGLE_CLIENT_IDS`）、exp、email_verified。
  - 用户绑定：先按 `google_sub` 找，再按 email 找并补绑；都没有则新建（password_hash 用哨兵 `!google-sign-in-only`，`email_verified_at=now`）。
  - 迁移 `0014_google_identity.sql`：`users.google_sub` + partial unique index。
  - `.env.example` 增加 `ONIA_GOOGLE_CLIENT_IDS`（空 = 禁用端点）。
- 客户端：
  - 依赖 `google_sign_in 7.2.0`；`ONIA_GOOGLE_SERVER_CLIENT_ID` dart-define（config/dev.json、config/prod.json 待填）。
  - 登录页新增「使用 Google 继续」+ 或分割线（`apps/client/lib/screens/login_screen.dart`）。
  - `runtime/google_auth.dart` 取 ID token，取消返回 null。
  - `session.loginWithGoogle` → `auth.google`。

## 验证

- 官网：build + rsync 到 `/opt/onia/web`；线上 index 指向 `index-BYz_gX2H.js`；headless Chrome 确认 `/privacy-policy`、`/terms-of-service` 渲染（部署前线上是旧包，SPA fallback 只返回空壳）。
- 服务端：`cargo test -p onia-server` 61 passed（google 模块 8 个：有效/issuer/aud/过期/未验证邮箱/未知 kid/未配置）。
- 客户端：`flutter analyze` 干净；`flutter test` 113 passed；`flutter build apk --debug --flavor dev` 成功，包名 `com.colna.onia.onia_client.dev`，minSdk 24，签名 SHA-1 同上。

## 待办

- 控制台：Branding、Production、测试用户、debug Android client、Web client。
- 服务端：填入 `ONIA_GOOGLE_CLIENT_IDS` 后构建镜像并部署（含迁移 0014）。
- 客户端：把 Web client ID 写进 `config/dev.json` / `prod.json`，重打包真机联调。
- iOS：如要支持，需要 iOS OAuth client + Info.plist URL scheme（本次只做 Android）。
