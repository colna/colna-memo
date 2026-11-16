---
title: Onia 旧 WebView 87 缺 Array.prototype.at
date: 2026-11-17
tags: troubleshooting, onia, android, webview
---

# Onia 旧 WebView 87 缺 Array.prototype.at

## 现象

Android 10 设备（System WebView 87.0.4280.101）上招呼发送失败，DB 里 `SEND_GREET` 结果为「API 发送失败: 未返回 message_id」；控制台同时出现 `r.entries.at is not a function`、Instagram 模块加载失败等错误。

## 根因

`Array.prototype.at` / `String.prototype.at` 是 Chrome 92+ 才有的 API。旧 WebView 执行页面脚本时直接抛错，监听和发送链路被中断，错误被上层折成「未返回 message_id」。

## 修法

- 页面脚本去掉 `.at(-1)`，改用索引访问。
- bundle 与 Android `onPageStarted` 尽早注入 `Array.prototype.at` / `String.prototype.at` polyfill。
- 读取 WebView 版本（Android native `getWebViewVersion`）；`<92` 自动 `preferDom=true`，`send_greet` / `send_message` 跳过 GraphQL API，直接走「主页 → Message → 输入 → 发送」DOM 路径。
- DOM 模式跳过旧 WebView 不稳定的 `assertLiveSession` / `requireViewer`。
- 根本修复仍是升级 System WebView 到 92+（建议当前版本）后重启 App。

## 验证

- `packages/scripts` 132 tests、Flutter 85 tests / analyze 通过。
- 新 APK 安装到 Android 10 测试机后，招呼链路恢复。

## 相关文件

- `packages/scripts/src/instagram/thread_sync.ts`
- `apps/client/lib/instagram/instagram_session.dart`
- `apps/client/android/app/src/main/kotlin/com/colna/onia/onia_client/OniaInstagramNativePlugin.kt`
