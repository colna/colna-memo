---
title: iOS 27 模拟器 App 启动即崩 —— UIScene 生命周期强制（sitin-rn / Expo SDK 57）
date: 2026-09-24
tags: troubleshooting, ios, expo, react-native, xcode, sitin-rn, simulator
---

# iOS 27 模拟器 App 启动即崩 —— UIScene 生命周期强制

场景：iPhone 18 Pro 模拟器（iOS 27.0 runtime）装 `apps/luka` 的 Debug 包（Xcode 27 编译），
启动即 SIGTRAP 闪退。同类问题会影响所有以 Xcode 27 编译、还没 opt-in scene 的 RN/Expo 包。

## 现象与定界

- `simctl launch` 返回 PID 但界面立刻回桌面；崩溃报告 `EXC_BREAKPOINT / SIGTRAP`，
  栈顶是 `UIKitCore ___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption`。
- 系统日志一句话点名根因：
  `Application failed to launch: UIScene life cycle is required for apps built with this SDK.`
- **判据是运行时的 iOS 版本**：同一个包在 iOS 26.5 模拟器/iPadOS 26.3 真机跑得好好的，
  在 iOS 27 运行时上必崩；deployment target 写多低都没用。

## 修法（Expo SDK 57）

1. catalog 的 `expo` 升到 `~57.0.23`（SDK 57 的 scene 运行时 backport，expo#50191）。
2. app 引入 `expo-build-properties`（≥ 57.0.20，属性在 expo#50205 加入），
   `plugins` 里加 `["expo-build-properties", { ios: { enableSceneSupport: true } }]`。
3. `prebuild -p ios --clean` → 确认 `ios/<App>/Info.plist` 有
   `UIApplicationSceneManifest`（`UISceneDelegateClassName = EXExpoAppSceneDelegate`），
   AppDelegate 改为实现 `ExpoReactNativeFactoryProvider`。
4. 重建原生包。升级 SDK 58 后该属性变 no-op，可移除。

**代价**：走 scene 生命周期后，直接挂在 `UIApplicationDelegate` 上的第三方钩子不再由
UIKit 触发（Expo 转发 scene 事件给 `ExpoAppDelegate` / `ExpoAppDelegateSubscriber`），
依赖 AppDelegate swizzle 的 SDK（TIM push、TRTC、AppsFlyer、BytePlus）需真机回归。

## 附带坑：iPhone 18 机型无法用 iOS 26.5 运行时绕过

`xcrun simctl create "iPhone 18 Pro" com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro
com.apple.CoreSimulator.SimRuntime.iOS-26-5` → `Incompatible device`（403）。iPhone 18
设备类型只随 iOS 27 运行时提供，所以「降级模拟器运行时」这条路对 iPhone 18 不通，
只能在 App 侧 opt-in scene。

## 附带坑：无头模拟器上点不掉「在 Luka 中打开?」

iOS 27 里 `simctl openurl` 打开自定义 scheme 会弹确认框，而 Simulator.app GUI 没开时
没人能点。绕过（Expo CLI 同款机制）：往设备写 scheme approval，再重启 SpringBoard：

```bash
PLIST=~/Library/Developer/CoreSimulator/Devices/<UDID>/data/Library/Preferences/com.apple.launchservices.schemeapproval.plist
# plist 键：com.apple.CoreSimulator.CoreSimulatorBridge--><scheme> = <bundle id>
# 需要 luka 与 exp+luka 两个 scheme 都写（expo run:ios 用 exp+luka://）
xcrun simctl spawn booted launchctl kickstart -k system/com.apple.SpringBoard
```

注意：`simctl kickstart` 重启 SpringBoard 期间 openurl 会超时，等 6–8 秒再发。
装/重装 App 后 approval 可能被重置，要重写。

## 验证

- `curl http://localhost:8081/json/list` 出现 `com.presence.gracechat (iPhone 18 Pro)`。
- `xcrun simctl io booted screenshot /tmp/x.png` 看屏幕（无头调试全靠它）。

## 相关

- 仓库内文档：`docs/native-dev-onboarding.md` §八 常见故障（已加条目）。
- 先例：`origin/feat/pogo` 的 `a4bc838ce`（pogo 包更早做过同一 opt-in，含 catalog 注释）。
- 姊妹篇：[[luka-house-id-device-build-usb-tunnel]]（房号真机包 + USB 隧道连 Metro）。
