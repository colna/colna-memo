---
title: SmartReply 客户端 SP 端能力与 PWA 桥兼容
date: 2026-10-29
tags: [sitin-next, app-pwa, smartreply, social-proxy, webview, bridge]
---

# SmartReply 客户端 SP 端能力与 PWA 桥兼容

2026-10-29 在 `sitin-next` 的 `packages/app-pwa` 接入 SmartReply Android 客户端。该客户端 UA 追加 `smartreply_android`，只提供 Social Proxy（SP）端能力，没有通话、相机、APK 升级等完整 APK 能力。

## 结论

- 不要扩大 `isApp()`：它继续只认 `window.pwaBridge`，否则 SmartReply 会被误判为完整 APK。
- SP 环境门禁用 `isSocialProxyHost()`；SP 桥调用统一走 `invokeSocialProxyBridge()`。
- `isInHostApp()` 纳入 `smartreply_android`，阻止 APK 下载/应用商店跳转和浏览器全屏接管。
- 桥对象兼容 `window.spBirdge`（客户端既有拼写）与 `window.spBridge`。
- 统一签名 `method(params, callbackId)`，响应信封 `{code, payload, reason}`，`callbackId` 原样回传。

## SP 端能力（18 个）

### 登录态（4，仅可信域）

仅允许 `app.sitin.ai`、`app-test.sitin.ai` 及其子域调用；其它域返回 `UNTRUSTED_ORIGIN`。

- `getLoginInfo` → `{token, tim_sig, user_id}`
- `updateLoginInfo({userId|user_id, token, username, timSig?, timAppId?})`
- `updateNetworkToken({token})`，空串表示登出
- `getPhoneDeviceId` → native deviceId

### Social Proxy 业务（11）

- `openInsWebView`
- `openSocialProxyWebview({platform, url, webView?, timeoutMs?})`
- `startInsRobot(StartInsData)`
- `finishInsRobot`
- `startSocialProxyRobot({platform, url, userId, mode, aiImage, aiStopImage, loginPathPatterns, userName})`
- `finishSocialProxyRobot({platform?})`
- `updateConversationState(会话状态 JSON)`
- `checkInsPageAbnormal({username})`
- `checkSocialProxyPageAbnormal({platform, url, webView?, timeoutMs?})`
- `testInsChatsView`
- `toggleBackgroundWaitingEarn({start: Boolean})`

### 权限 / 连通性（3）

- `checkPermission("Floating")` → 是否有悬浮窗权限
- `requestPermission("Floating")` → 结果回调 `window.finishFloatingPermission('true'|'false')`
- `ping` → `pong`

### 端 → JS

- `window.finishPWAInsTask(state, insId, insAvatar, platform?)`
- `window.openSocialLogin({platform})`
- `window.finishFloatingPermission(...)`

## PWA 接入要点

- `invokeSocialProxyBridge()`：完整 APK 走 `pwaBridge`；SmartReply 优先直接方法，兼容 `callHandler`、同步返回/Promise、`window[callbackId]`、`window.spBridgeCallback`。
- `userStore` 冷启动恢复从 `isApp()` 改为 `isSocialProxyHost()`，SmartReply 可通过 `getLoginInfo` 恢复登录态。
- `getPhoneDeviceId`、`updateLoginInfo`、`updateNetworkToken`、`updateConversationState` 均走 SP 桥。
- 悬浮窗权限：用 `hasSocialProxyMethod("checkPermission")` 做能力探测，有则优先 `checkPermission`，否则走 `isPermissionGranted`（不靠 UA 判断方法名）。
- `startSocialProxyRobot` 补 `userName`；`checkSocialProxyPageAbnormal` 参数补 `timeoutMs`。
- 埋点 `source` 用 `isFemalePackageHost()` 判定，包含 APK / haven_pwa / savvy，**明确排除 `smartreply_android`**；`isInHostApp()` 不再兼任埋点口径。
- 仅保留实际调用到的封装与 `hasSocialProxyMethod`。无调用点的 `finishSocialProxyRobot` / `finishInsRobot` / `testInsChatsView` / `toggleBackgroundWaitingEarn` / `ping` 暂不封装，需要时再按同一模式加。
- `updateLoginInfo` 失败时降级调用 `updateNetworkToken`，至少同步 token。

## 验证与风险

- lint、tsc、Prettier（改动文件）、`pnpm test`（65 tasks）、full build（65 tasks）通过。
- Playwright 模拟 `smartreply_android` UA + 18 个 `spBirdge` 方法：路由、callbackId、checkPermission、缺失方法 -501 降级全部通过。
- 未接 SmartReply 真机/APK；上线前需客户端注入 `spBirdge`，并在可信域联调登录态。

实现文档：`sitin-next/packages/app-pwa/docs/smartreply-sp-bridge.md`。

## 2026-10-30 CR 修复

MR !1319 的 4 条未解决 CR 均已修复并 resolve（commit `61a5c31af`）：

1. **埋点口径**：新增 `isFemalePackageHost()`，避免 `isInHostApp()` 扩大后把 SmartReply 任务完成记成 `female_package`。
2. **响应误判**：`toBridgeResponse` 空串 → `-502`、非 JSON → `-502`、非信封对象 → `-503`，与 `invokeBridge` 语义对齐。
3. **APK 超时**：`invokeBridge(method, params, timeoutMs = 0)`，`invokeSocialProxyBridge` 透传；默认 0 保证 GraceChat 旧调用无超时、行为不变。
4. **可读性 / 死代码 / 日志**：拆 `acceptButtonText()`；删除无调用点封装；reason/log 用 `spBridge`，仅保留 `window.spBirdge` 属性名对齐端契约。

## 过渡期结束后的收敛

1. SmartReply 新版本注入 `pwaBridge`（内部适配现有 SP 实现），`isApp()` 自动为 true。
2. 删除 SmartReply 能力特判：`useInsTaskInit.ts` 的 `isSmartReplyClient() ||`、权限方法名特判等。
3. 旧版本占比清零后删除 `invokeSpBridgeMethod` / `getSpBridge` / `spBirdge` 直连，`invokeSocialProxyBridge` 收敛为 `invokeBridge` 薄封装。
4. 保留 `isInHostApp()`（含 smartreply_android）、`isFemalePackageHost()`（排除 smartreply_android）和 UA 识别。
5. 回归矩阵：GraceChat APK（pwaBridge）、SmartReply 新版本（pwaBridge）、SmartReply 旧版本（spBirdge）、普通浏览器（不调原生）。

## 双路径回归

Playwright 23 项检查通过：SmartReply `spBirdge`（环境/设备 ID/登录态/权限/页面检测/启动浮窗/登录态降级/会话状态）、协议错误（非 JSON `-502`、空串 `-502`、非信封 `-503`）、GraceChat `pwaBridge`（原方法路径、`isPermissionGranted`、超时 `-504`）、埋点（SmartReply → `pwa`，GraceChat → `female_package`）。
