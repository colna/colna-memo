---
title: RN 预编译 core 与预编译 Expo 模块混搭 → 启动即 dyld 崩溃（模拟器也会中）
date: 2026-09-15
tags: troubleshooting, react-native, ios, cocoapods, expo, dyld, sitin-rn
---

# `dyld: Library not loaded: @rpath/React.framework/React`（模拟器冷启动）

出处：sitin-rn `apps/luka`，2026-09-15。团队此前只在**真机**上遇到过（见
`apps/iris/docs/05-native-capabilities.md`），这次在**模拟器**上复现 —— 因为触发条件不是
「真机」，而是 **pod install 那一刻 Maven 拉不到预编译产物**。

## 症状

- app 装到模拟器上，**点开就闪退**；`simctl launch` 返回 PID 但进程立刻没了。
- 崩溃报告（`~/Library/Logs/DiagnosticReports/Luka-*.ips`）：

  ```text
  termination: Library missing
  reasons: Library not loaded: @rpath/React.framework/React
           Referenced from: .../Luka.app/Frameworks/ExpoVideo.framework/ExpoVideo
  ```

- 关键证据：`Luka.app/Frameworks/` 里**没有** `React.framework` /
  `ReactNativeDependencies.framework`，但 `ExpoVideo.framework`（Expo 预编译 XCFramework）
  依赖它们。

## 根因

SDK 54+ 的 Podfile 默认 `RCT_USE_PREBUILT_RNCORE ||= '1'`、`EXPO_USE_PRECOMPILED_MODULES ||= '1'`，
两者本应配套：Expo 模块是**预编译动态框架**，通过 `@rpath` 找**预编译的** `React.framework`。

`ReactNativeCoreUtils.setup_rncore` 会先做一次 **Maven 产物存在性检查**
（`repo1.maven.org/.../react-native-artifacts-<ver>-reactnative-core-debug.tar.gz`）。
检查失败（网络/代理）时它**静默回落**：

```text
[ReactNativeCore] No prebuilt artifacts found, reverting to building from source.
[ReactNativeCore] Building from source: true
```

于是 RN core 从源码编译（`Podfile.lock` 里是 `React-Core`、不是 `React-Core-prebuilt`），
而 Expo 模块仍是预编译的 —— **混搭**：预编译模块要动态 `React.framework`，源码编译路线
把它静态链进 app 二进制、不产这个框架。运行时 `@rpath` 找不到，dyld 直接杀。

## 判断口径（30 秒）

```sh
cd apps/<app>/ios
grep -c "React-Core-prebuilt" Podfile.lock        # 0 → 落到了源码编译
grep "React.framework" "Pods/Target Support Files/Pods-<App>/Pods-<App>-frameworks.sh"   # 没有 → 不会 embed
```

pod install 的输出里也有一行 `[ReactNativeCore] Building from source: true/false`，
**当场就能看见**，不用等构建。

## 修法

1. 确认 `https://repo1.maven.org/maven2/` 可达（本机 2026-09-15 是 Clash 代理开着时不通，
   关掉/换节点后正常；注意 `cdn.cocoapods.org` 也会同时抽风）。
2. **重跑 `pod install`**（不需要 `--clean` prebuild）—— 它会重新解析到预编译 core，
   写回 `Podfile.lock` 与 `Pods-<App>-frameworks.sh`。
3. 重新构建。产物里应出现 `React.framework` + `ReactNativeDependencies.framework`。

另一条确定性路线（iris/koda/lumi 的 `ios:device` 用的）：`EXPO_USE_PRECOMPILED_MODULES=0`
（且必须在 pod install 发生时在场），全部从源码编译，运行时不存在这个动态依赖 —— 代价是
构建慢一截。

## 相关

- 真机版的同一坑与两个「环境变量必须在场」的细节：`apps/iris/docs/05-native-capabilities.md`
- 本机网关被地域拦截时需要代理：见 [muskapi-image2-relay](muskapi-image2-relay.md) ——
  **代理与 pod install 的 Maven 检查会互相干扰**，出包前先确认网络状态。
