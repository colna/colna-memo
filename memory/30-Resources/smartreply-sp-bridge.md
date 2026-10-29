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

## SP 端能力

JS → 客户端：

| 方法 | 作用 |
|------|------|
| `startSocialProxyRobot` | 启动/复用社媒运营浮窗 |
| `openSocialProxyWebview` | 打开社媒登录/重登页 |
| `checkSocialProxyPageAbnormal` | 检测登录态/页面异常 |
| `isPermissionGranted("Floating")` | 查询悬浮窗权限 |
| `requestPermission("Floating")` | 申请悬浮窗权限 |

客户端 → JS：

| 回调 | 作用 |
|------|------|
| `window.finishPWAInsTask(state, insId, insAvatar, platform?)` | 授权完成 |
| `window.openSocialLogin({ platform })` | 通知 PWA 处理登录态异常 |
| `window.finishFloatingPermission(...)` | 系统设置返回后继续权限流程 |

旧入口降级：`startInsRobot`、`openInsWebView`、`checkInsPageAbnormal`。

## 桥协议兼容

同时支持 `callHandler(method, params, callback)`、直接方法、同步返回/Promise、`callbackId` + `window[callbackId]`、`window.spBridgeCallback(callbackId, response)`。响应结构 `{ code, reason, payload }`，`code = 0` 为成功。

## 验证与风险

- lint、Prettier、circular、`pnpm test`（65 tasks）、full build（65 tasks）通过。
- Playwright 模拟 `smartreply_android` UA 验证环境判断、5 个 SP 方法路由及多种回调通路。
- 未接 SmartReply 真机/APK；上线前需客户端按上述方法名注入 `spBirdge` 并联调。

实现文档：`sitin-next/packages/app-pwa/docs/smartreply-sp-bridge.md`。
