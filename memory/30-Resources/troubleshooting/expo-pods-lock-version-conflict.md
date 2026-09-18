---
title: 拉取原生依赖变更后的 pod install：ExpoModulesCore 版本冲突与连环 update
date: 2026-09-18
tags: troubleshooting, expo, react-native, cocoapods, ios, pods
---

# `pod install` 报 path pod「版本不兼容」：逐个 update 会连环，要一起 update

**症状**（2026-09-18，Luka，拉取含原生依赖变更的分支后跑 `pnpm luka:ios`）：

```
[!] CocoaPods could not find compatible versions for pod "ExpoModulesCore":
  In snapshot (Podfile.lock):
    ExpoModulesCore (from `../../../node_modules/expo-modules-core/ExpoModulesCore.podspec`)
  In Podfile:
    ExpoModulesCore (from `../../../node_modules/expo-modules-core/ExpoModulesCore.podspec`)
It seems like you've changed the version of the dependency `ExpoModulesCore` and it
differs from the version stored in `Pods/Local Podspecs`.
You should run `pod update ExpoModulesCore --no-repo-update` to apply changes made locally.
```

## 什么时候会遇到

分支改动里 **`expo-modules-core` 或其它 path pod 的版本变了**（例如 `pnpm-workspace.yaml`
catalog 钉了版本、或新增了原生依赖），本机 `ios/` 是旧一版生成的。`pnpm install` 装完
node_modules 之后，`ios/Podfile.lock` 与 `Pods/Local Podspecs` 都还是旧的版本快照。

## 为什么 `pod install` 修不好

`pod install` **尊重 `Podfile.lock`**：它把 lock 里记录的 path pod 版本当约束，而
node_modules 里 podspec 的版本已经变了，于是永远「找不到兼容版本」。

两条看起来对、实际会连环或无效的修法：

| 尝试 | 结果 |
| --- | --- |
| `pod update ExpoModulesCore` | 修好一个，下一个同包 pod 接着报（`ExpoModulesWorklets` → `ExpoModulesWorkletsAdapter`） |
| `rm -rf Pods/Local Podspecs && pod install` | 还是被 `Podfile.lock` 拦，报同一个错 |

## 正确修法：一次 update 同一包的全部 podspec

`expo-modules-core` 这一个包带多个 podspec，版本是一起变的，所以要一起 update：

```bash
cd apps/<app>/ios
pod update ExpoModulesCore ExpoModulesWorklets ExpoModulesWorkletsAdapter --no-repo-update
```

- `--no-repo-update`：只用本地 specs 缓存，不拉整个 CocoaPods CDN。
- 不知道同包有哪些 podspec：`ls node_modules/<包>/*.podspec`。
- 兜底（更重）：删 `Podfile.lock` 后 `pod install` 让 lock 全量重建——会重新解析所有
  registry pod，可能引入额外漂移，不是首选；`expo prebuild --clean` 最彻底但最慢。

update 成功后会看到 `ExpoCamera` 等新 pod 被写入 `Podfile.lock`（`grep ExpoCamera Podfile.lock`），
以及正常的 script phases 提示，**不是错误**。

## 验证

```bash
grep -n "ExpoCamera\|ExpoModulesCore (" apps/<app>/ios/Podfile.lock
ls -d apps/<app>/ios/Pods/ExpoCamera
```

## 相关

- [缺原生模块时 expo-* 的 import 期抛错](expo-missing-native-module-guard.md)（装完 JS 依赖后旧 dev client 还缺原生模块）
- [Metro bundle 热 + 文件写入的坑](metro-bundle-hot-and-file-write.md)
