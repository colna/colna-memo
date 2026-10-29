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
- 悬浮窗权限：SmartReply 优先 `checkPermission`，完整 APK 优先 `isPermissionGranted`。
- `startSocialProxyRobot` 补 `userName`；`checkSocialProxyPageAbnormal` 参数补 `timeoutMs`。
- 新增封装：`finishSocialProxyRobot`、`finishInsRobot`、`testInsChatsView`、`toggleBackgroundWaitingEarn`、`pingSocialProxy`、`hasSocialProxyMethod`。

## 验证与风险

- lint、tsc、Prettier（改动文件）、`pnpm test`（65 tasks）、full build（65 tasks）通过。
- Playwright 模拟 `smartreply_android` UA + 18 个 `spBirdge` 方法：路由、callbackId、checkPermission、缺失方法 -501 降级全部通过。
- 未接 SmartReply 真机/APK；上线前需客户端注入 `spBirdge`，并在可信域联调登录态。

实现文档：`sitin-next/packages/app-pwa/docs/smartreply-sp-bridge.md`。
