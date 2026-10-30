---
title: Android WebView JavaBridge "Method not found"（spBridge）
date: 2026-10-30
tags: [android, webview, jsbridge, smartreply, sitin-next]
---

# Android WebView JavaBridge "Method not found"

## 现象

PWA 内调用 `window.spBridge.getPhoneDeviceId(...)` 报：

```
Bridge 调用 spBridge 方法 getPhoneDeviceId 失败: Error: Error invoking getPhoneDeviceId: Method not found
```

## 根因

`addJavascriptInterface` 暴露的 Java 方法按 **参数签名** 精确查找：

```kotlin
@JavascriptInterface
fun getPhoneDeviceId(params: String, callbackId: String)   // (String, String)V
```

前端却传了三个参数：

```ts
handler.call(bridge, serialized, callback, callbackId)    // (String, Object, String)
```

Java 侧找不到匹配签名 → `Method not found`。注意 `typeof bridge[method] === "function"` 仍为 true，能力探测拦不住这类错误。

## 正确调用方式

客户端的 `WebViewJavascriptBridge.js` 里 `callHandler` 才是标准入口：

```js
spBridge.callHandler(method, params, responseCallback)
```

它内部会：

1. 生成 `callbackId` 并写入闭包内的 `responseCallbacks`；
2. `fn.call(spBridge, message, callbackId)` —— 只传两个参数；
3. 原生 `sendResponse(callbackId)` → `_handleMessageFromNative` → `responseCallbacks[callbackId]` 回调。

直接调 Java 方法（两个参数）虽然不报错，但**响应会丢**，因为 `responseCallbacks` 里没有这个 id。

## 修复

- `invokeSocialProxyBridge` 优先 `spBridge.callHandler`；
- 直接方法兜底只传 `(params, callbackId)`；
- 冷启动等 `callHandler` 就绪（最长 3s），因为 JS 桥在 `onPageFinished` 才注入。

## 排查要点

- `Method not found` ≈ 参数个数/类型不匹配，或 APK 里确实没有该方法（旧包）；
- 响应丢失先看 `responseCallbacks` 是否有对应 `callbackId`；
- 端能力方法本身在 `addJavascriptInterface` 后即可调用，不依赖 JS 注入；依赖注入的只有 `init/registerHandler/callHandler`。
