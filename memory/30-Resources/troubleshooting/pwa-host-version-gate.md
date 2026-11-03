---
title: PWA 共享代码用宿主版本号做能力门 —— 跨宿主版号不可比（earningai 1.0.0 / savvy 1.34）
date: 2026-11-03
tags: [troubleshooting, pwa, jsbridge, earningai, 悬浮窗, 能力探测]
---

# 用 `getApkVersion()` 当能力门 → 新宿主永远走旧路径

## 现象

`earning-ai-android`（`release/v1.0.0`）冷启动后：不弹「Allow System Access」悬浮窗授权弹窗，也拉不起小崽；同一套 PWA 代码在 savvy 上正常。

## 根因

`packages/app-pwa/src/hooks/useInsTaskInit.ts:205-206` 用版号判断走新/旧社媒路径：

```ts
isNewVersion = getApkVersion() >= 1.34
```

- earningai 端 `BuildConfig.VERSION_NAME = 1.0.0`，`DeviceHandler.kt:32-34` 原样返回 → `1.0.0 < 1.34`，永远判旧。
- 旧路径里 `if (insAbnormal) return;`，且 `getPwaFollowedUser()` 拿不到 followed 用户就直接 return → `guideFloatingPermissionIfIgBound()` / `startRobot` **全部跑不到**。
- 即：能力和版号无关，**版号语义各宿主独立**，savvy 的 `1.34` 不能外推到 earningai 的 `1.0.0`。跨宿主共享 PWA 代码用版号做门，等于把「savvy 的发布节奏」硬编码成全局能力表。

## 修法：版本门 → 端能力门

```ts
const hasNewSocialBridge =
  hasNativeMethod("checkSocialProxyPageAbnormal") && hasNativeMethod("startSocialProxyRobot");
const isNewVersion = hasNewSocialBridge || version >= 1.34;   // 保留版号兜底
```

earningai v1.0.0 两个方法都有（`SocialHandler.kt:129` / `:73`），改后立刻走新路径。落地：`f46469c37`（1 文件 +8/−1），MR !1360，已合 `release/test-pwa2`。

## 配套：权限授予后不会自动拉起（回前台补一次）

新路径本身只负责「弹窗引导」。用户去系统设置授权后回前台，原 `startRobot` 已过 30s 窗口放弃 → 小崽仍不起。补一层 `visibilitychange`（`5a386f14f`，1 文件 +42/−1）：回前台且 `isApp`、本次会话 `miniShown.instagram !== true`、`selectPlatformUsable(IG)`、`checkFloatingPermission()==="granted"` 时，延迟 1.2s 再 `startRobot(EMPTY_ROBOT_MSG)`；用 `floatingRetryRef` 防并发、`miniShown` 防重复拉起/不打扰用户主动收起。

## 真机验证（earningai release/v1.0.0 debug，R92W902WQ6K）

```bash
# 端口：宿主 3000 被占时不要同端口 reverse（见 pwa-localhost-port-conflict.md）
adb reverse tcp:3000 tcp:3001
pnpm --filter @heyhru/app-pwa dev -- --port 3001

# 造数：长按 Brands 标题触发 debug seedReady()（profileComplete / linkedInstagram=true）
node .../login-with-session.mjs --sessionId ... --socket webview_devtools_remote_<socialweb pid>

# 权限门反复开关
adb shell appops set com.pwa.app SYSTEM_ALERT_WINDOW deny
# 清 ins-storage 后 reload → 应弹「Allow System Access」
# 等 >30s（确认原 startRobot 已超时放弃）→
adb shell appops set com.pwa.app SYSTEM_ALERT_WINDOW allow
# Home + 回前台 → miniShown.instagram=true，出现 135×135 APPLICATION_OVERLAY 窗口
```

判据：`checkConnectedRecord(<userId>,1)=false` + `hasContactCard(1)=true`；清空 `ins-storage` 重载后应写回 `authByPlatform.instagram=true / authEvidenceByPlatform.instagram=true / abnormal=false`。

## 教训

1. 共享 PWA 代码里任何 `getApkVersion()` 分支都要假定「跨宿主不可比」；能探测能力就用 `hasNativeMethod`，版号只作兜底。
2. 端能力名要以端侧 `@JavascriptInterface` 原文为准（`SocialHandler.kt`），不要按需求文档猜。
3. 「引导用户去系统设置授权」必须配一条「回前台补拉」路径，否则授予权限也不生效。

## 遗留

权限授予后「Allow System Access」弹窗不自动关闭（小崽已在背后起来）；建议 `showInsAuthPermissionModalAsync`（`InsAuthPermissionModal.tsx:133`）在 granted 后自动 close。
