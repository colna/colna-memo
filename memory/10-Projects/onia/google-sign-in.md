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
  - `Onia Web`（官网/控制台 GIS 用）：client ID `328960377279-salium6oef7gll6qkibqm62n99qcibcc.apps.googleusercontent.com`；JS 来源 onia.asia / admin.onia.asia / localhost:5173；重定向 URI 含 api.onia.asia/v1/auth/google/callback（当前不用）。
  - `Onia App Browser`（新建，Desktop app 类型）：client ID `328960377279-6tgk506lkd92fntf3lmi07umde61k3g1.apps.googleusercontent.com`；App 浏览器 OAuth（loopback + PKCE）用，不需要 client secret。
- 旧 release 包与 dev 包目前都用 debug keystore 签名，所以两个 Android client 可共用同一 SHA-1；换正式签名后要补新指纹。

## 配置

- `apps/client/config/dev.json`、`config/prod.json` 已加 `ONIA_GOOGLE_OAUTH_CLIENT_ID=<Onia App Browser client ID>`（旧的 `ONIA_GOOGLE_SERVER_CLIENT_ID` 已移除）。
- 本地与生产 `.env` 的 `ONIA_GOOGLE_CLIENT_IDS` 已包含 Web + Desktop 两个 client ID。
- 服务端 `ONIA_GOOGLE_CLIENT_IDS` 为 `aud` 白名单；空则 `/v1/auth/google` 返回 422「not configured」。

## 代码实现

- 协议：新增 kind `auth.google` / `auth.google.ok` 与 `AuthGooglePayload { id_token, locale }`（crates/onia-protocol）。
- 服务端：
  - `/v1/auth/google`（crates/onia-server/src/http/auth.rs）。
  - `GoogleVerifier`（crates/onia-server/src/google.rs）：JWKS（`https://www.googleapis.com/oauth2/v3/certs`，1h 缓存、未知 kid 刷新一次）、RS256、iss（两种写法）、aud、exp、email_verified。
  - 用户绑定：先按 `google_sub` 找，再按 email 找并补绑；都没有则新建（password_hash 用哨兵 `!google-sign-in-only`，`email_verified_at=now`）。
  - 迁移 `0014_google_identity.sql`：`users.google_sub` + partial unique index。
- 客户端：
  - 依赖 `url_launcher`（已移除 `google_sign_in`）；登录页「使用 Google 继续」启动系统浏览器。
  - `runtime/google_auth.dart`：Desktop client + OAuth authorization code + PKCE + 本地 loopback HttpServer（`http://127.0.0.1:<随机端口>`）接收回调；用 code 换 `id_token`，取消/超时返回 null。不需要手机系统里有 Google 账户。
  - `session.loginWithGoogle` → `auth.google`。
  - 登录页改为 Google 优先：默认只显示「使用 Google 继续」+「使用其他方式登录」+「创建账户」；点前者展开邮箱/密码表单（服务地址也只在展开后显示），并有「返回 Google 登录」收回。新增 1 个 widget 测试覆盖默认/切换。
  - 登录页合法链接固定底部：服务条款/隐私政策从滚动内容移到页面底部，`SafeArea(top: false)` + 12px 最小底部间距。
  - 手动输入时整屏上移：键盘弹出且处于邮箱/密码模式时，`AnimatedSize` 把顶部 Hero 收到 0 高度，表单获得整屏空间；收起键盘恢复，底部合法链接在输入时隐藏。

## 验证

- 官网：build + rsync 到 `/opt/onia/web`；headless Chrome 确认 `/privacy-policy`、`/terms-of-service` 渲染。
- 服务端：`cargo test -p onia-server` 61 passed（google 模块 8 个）。
- 本地服务端：重启新二进制后 POST `/v1/auth/google` + 假 token 返回 401 `invalid google token`（路由与配置生效）。
- 浏览器 OAuth：OPPO 真机点「使用 Google 继续」能拉起系统浏览器（HeyTap Browser）；但该机无 VPN，`accounts.google.com` 不可达，页面停在加载中，待开启 VPN 后完成登录验证。
- 客户端：`flutter analyze` 干净；`flutter test` 113 passed；dev debug APK 已用带 client ID 的 `config/dev.json` 重新构建。
- 本地 dev API：`http://192.168.3.24:8080`；手机 serial `f7031061` 在线。

## 官网 Google 注册（2026-11-18 19:11）

- `https://onia.asia/register` 增加官方 Google 按钮（Google Identity Services，`google.accounts.id.renderButton`，`text=signup_with`，outline/pill），按钮下方是“使用 Google 注册即表示同意服务条款与隐私政策”+「或」分隔线，再往下是原邮箱注册表单。
- 回调拿 `credential`（ID token）后调用 `POST /v1/auth/google`（复用服务端端点），成功后展示成功页：Google 用户隐藏「打开后台」按钮，提示用同一 Google 账号在 App 登录（Google 账号没有密码，后台仍走邮箱密码）。
- 前端实现：`apps/web/src/api.ts` 新增 `registerWithGoogle`；`Register.tsx` 动态加载 `https://accounts.google.com/gsi/client`，按当前语言初始化/渲染按钮；`google-identity.d.ts` 补最小类型；`copy.ts` 中英文案；`styles.css` 新增 Google/分隔线样式。
- client ID：复用 Web client `328960377279-salium6oef7gll6qkibqm62n99qcibcc`（可用 `VITE_GOOGLE_CLIENT_ID` 覆盖）。已授权 JS 来源含 `https://onia.asia`，现场按钮正常渲染；`http://localhost:5174` 未加入授权来源，本地 dev 若要测需先在控制台加。
- 验证：`pnpm --filter @onia/web typecheck` + `build` 通过；rsync `dist/` 到生产 `/opt/onia/web`；headless Chrome 实测线上 `/register` 出现 GIS iframe（client_id 正确）且布局正常。

## 控制台 Google 登录 + 改名（2026-11-18 19:52）

- 后台改名为「控制台」：网站 nav/footer/注册页文案（EN Console / ZH 控制台）、后台文档标题与 Umi title（`Onia 控制台` / `Onia Console`）、登录页文案、导航 aria-label、发布页文案、网站与 App 法律文本（`admin console` → `console`，`管理后台` → `控制台`）。域名 `admin.onia.asia` 与代码标识符不变。
- 控制台登录页支持 Google：邮箱密码表单下方「或」+ 官方 GIS 登录按钮（`text=signin_with`，Web client ID，locale 跟随控制台语言），回调 `POST /v1/auth/google` 后 `saveSession` 并跳 `/overview`；不经过邮箱验证码步骤。
- 验证：admin `umi build` + `tsc --noEmit` 通过；dist rsync 到 `/opt/onia/admin`；headless Chrome 确认 `https://admin.onia.asia/login` 渲染 GIS iframe（client_id 正确）、标题 `Onia 控制台`，截图布局正常。
- 备注：admin 部署使用了当前工作树，包含另一会话未提交的 TaskWorkflow / 筛选等改动；如需干净发布可回退重发。App 法律文案已同步改「控制台」，APK 已重建但设备断开未安装。

## 生产部署（2026-11-18 17:56）

- 本地 `docker buildx --platform linux/amd64 --pull=false` 构建 `onia-server:latest`（image id `7f1a00e7866d`，37.7MB gz），临时 context 只含 Cargo.toml/Cargo.lock/crates/apps/server + 临时 Dockerfile（rust:1.85-bookworm → debian:12-slim）。
- SSH 到 47.250.210.196 间歇性 KEX 超时；用 macOS 自带 rsync 的 `--partial --append` 重试 4 次传完，sha256 一致；用 `aliyun swas-open RunCommand` 校验 + `docker load` + 改 `/opt/onia/.env` + `docker compose up -d --force-recreate onia-server`。
- 迁移 `0014` success；`users.google_sub` 存在；`https://api.onia.asia/v1/auth/google` 假 token 返回 401 `invalid google token`；`/health` ok。
- 坑：macOS 自带 rsync 2.6.9 不支持 `--append-verify`，用 `--append`。

## 踩坑

- `.env` 的 `ONIA_EMAIL_FROM=Onia <no-reply@onia.asia>` 未加引号，`dotenvy::from_path` 解析失败（main.rs 忽略了错误），导致该行之后的变量（含 `ONIA_GOOGLE_CLIENT_IDS`）都不加载。已改为带引号。详见 [dotenvy-unquoted-value-stops-loading.md](../../30-Resources/troubleshooting/dotenvy-unquoted-value-stops-loading.md)。
- Chrome 控制台页面用无障碍树操作时，保存后 webarea 会从树里消失、窗口 inventory 也会不稳定；改用 Chrome AppleScript `execute javascript` 直接操作 DOM 才稳定完成创建 client。
- 国内本机 server 拉 Google JWKS 需代理：直连超时会导致 `could not fetch google certificates` → 401 `invalid google token`。本地启动加 `HTTPS_PROXY=http://127.0.0.1:7897`；生产（马来西亚）直连 Google 正常。详见 [reqwest-google-jwks-proxy.md](../../30-Resources/troubleshooting/reqwest-google-jwks-proxy.md)。
- Android 的 Google 登录是系统 Credential Manager 原生弹窗（账号选择），不是浏览器跳转；浏览器跳转属于 OAuth authorization code 流程，当前不需要。

## 待办

- 真机：dev APK 已安装并启动（serial `f7031061`，Wi-Fi `presence-5g` 192.168.3.146，同网段可达 192.168.3.24:8080）；等用户解锁手机后点「使用 Google 继续」，确认能建号/登录并进入工作台。
- 代码未 commit；iOS 需要单独的 iOS OAuth client + URL scheme。
- 备注：dev 包指向本地 `http://192.168.3.24:8080`；prod 包指向 `https://api.onia.asia`。生产环境未配置 chat agent（mock），与本次无关。
