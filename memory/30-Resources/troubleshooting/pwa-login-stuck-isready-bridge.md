---
title: PWA 登录成功但停在登录页 — isReady 被 native 恢复阻塞
date: 2026-10-30
tags: [sitin-next, app-pwa, smartreply, webview, bridge, login, isready]
---

# PWA 登录成功但停在登录页：isReady 被 native 恢复阻塞

## 现象

SmartReply / APK WebView 内点 Quick Login（或手机号/Google 登录）：

- `FastLoginRequest` 返回 200，`haven_token`、`auth-storage.userInfo/userState=FullRegister` 都已更新；
- 页面仍停在 `/onboarding`，按钮可继续点，但进不去 App。

## 根因

`useAuthNavigation` / `AppContent initUser` 都以 `useUserStore.isReady` 为门禁。登录成功只调用 `setToken` / `setUserInfo`，**不会把 `isReady` 置真**。

冷启动且无 PWA token 时，`userStore` 的 `onRehydrateStorage` 会走 `syncLoginFromNative()`：

```ts
if (!havenToken && isSocialProxyHost() && !sessionStorage.getItem(NATIVE_LOGIN_ATTEMPTED)) {
  void syncLoginFromNative();
  return;
}
state.isReady = true; // 走不到
```

`syncLoginFromNative` 先 `getLoginInfoFromNative()`（5 次、每次最长 15s），成功/失败后才在 `finally` 里 `setState({ isReady: true })`。这期间手动登录成功也无法放行，因为 `isReady` 仍为 false。

桥本身在 SmartReply 里还可能初始化失败：

- `BridgeWebViewClient.onPageFinished` 才注入 `WebViewJavascriptBridge.js`；
- `initBridge()` 只监听一次 `PwaBridgeReady`；若事件先于监听触发（或注入两次/SPA 重定向），`init()` 永不执行；
- 之后 `spBridge.callHandler` 在 `_doSend` 里访问未初始化的 `messagingIframe`，报 `Cannot set properties of undefined (setting 'src')`；native 侧同时报 `spBridge._handleMessageFromNative is not a function`。

## 修法

1. **登录成功立即置 ready**：在 `handleQuickLogin` / `leadFastLogin` 成功后 `useUserStore.setState({ isReady: true })`，或在 `setUserInfo` 里一并置 `isReady: true`（有 userInfo 就代表会话已确定）。
2. **hydration 用 setState**：`onRehydrateStorage` 里把 `state.isReady = true` 改成 `useUserStore.setState({ isReady: true })`，确保 React 订阅者收到通知并持久化。
3. **桥握手兜底**：`connectWebViewJavascriptBridge` 同时把 callback 塞进 `window.WVJBCallbacks`（jsbridge 原生支持），并/或轮询 `window.spBridge.inited`，避免错过一次性 `PwaBridgeReady`。
4. **缩短/解耦 native 恢复**：`getLoginInfo` 超时降到 2-3s、次数降到 1-2 次；或先置 `isReady=true` 再做 native 恢复（恢复成功再 setUserInfo）。
5. Android 侧可考虑更早注入 JS 桥（`onPageStarted`）或只注入一次。

## 排查命令

```bash
adb forward tcp:9333 localabstract:webview_devtools_remote_<pid>
curl -s http://127.0.0.1:9333/json
# 用 CDP 读 React fiber 里的 useUserStore 快照 / localStorage / spBridge.inited
```

## 实例

`smart-replay-android` 1.1.3 + `app-test3.sitin.ai`：Quick Login 200 后 116s 仍 `isReady=false`；正常 reload 后 `spBridge.inited=true`、`spBridge.callHandler('ping')` 返回 `pong`。
