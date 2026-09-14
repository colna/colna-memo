---
title: iOS 桌面小组件白屏：帧图只认 asset catalog，散文件静默失败
date: 2026-09-14
tags: [troubleshooting, react-native, ios, widget, expo, sitin-rn]
---

# iOS 桌面小组件白屏：帧图只认 asset catalog，散文件静默失败

出处：sitin-rn `packages/rn-home-widget`（Luka 的 HomeWidget 扩展），2026-09-14。
移植自 nugget 分支的 `2acd581c`（那个包在真机上先踩到）。

## 症状

- 桌面上组件整块**白/灰**，App 不崩、构建不报错，界面上没有任何提示。
- 唯一的证据在系统日志里：

  ```sh
  xcrun simctl spawn booted log show --last 10m --predicate 'processImagePath CONTAINS "HomeWidget"' --style compact \
    | grep -i "No image"
  # No image named 'frame-medium-00' found in asset catalog for .../HomeWidget.appex
  ```

- 只影响 iOS；Android 的帧图是 drawable，走的是另一条路。

## 根因

SwiftUI 的 `Image("frame-medium-00")` **只查 asset catalog**（`Assets.xcassets` → 编译成
`Assets.car`）。插件早先把帧图当**散文件**拷进扩展根目录（`frame-medium-00.jpg`），
`Image(name)` 找不到它，而且失败是静默的 —— 视图画成空，只剩
`.containerBackground` 那层底，看起来就是白/灰一块。

## 修法

插件改为生成 asset catalog：一帧一个 imageset（单倍图，命名与 Swift 的
`String(format: "%02d")` 对齐），把 `Assets.xcassets` 加进扩展的 Resources 构建阶段。
另加 `.contentMarginsDisabled()`（iOS 17+ API），否则帧图四周还会露一圈系统内容边距
的灰边；代价是**扩展最低系统 15.1 → 17.0**，iOS 16.x 机器上没有这个组件。

## 两个施工坑

1. **普通 `expo prebuild` 不会更新已存在的 widget target。** 生成器对同名 target 是
   幂等的（直接 return），构建阶段还是上一版的清单。改了插件必须
   `expo prebuild -p ios --clean`。
2. **`--clean` 可能被 Finder 的 `.DS_Store` 抢写而失败**：报
   `ENOTEMPTY: directory not empty, rmdir '.../ios/build/Build/Products/Debug-iphoneos'`
   （Finder 在删除过程中往目录里写了 `.DS_Store`）。删掉 `ios/build`（派生数据）再重跑
   `--clean` 即可。

## 验证（不用等真机）

- 产物里必须有 `Assets.car`，且帧名对得上：

  ```sh
  APPEX=$(xcrun simctl get_app_container booted <bundle-id>)/PlugIns/HomeWidget.appex
  xcrun --sdk iphonesimulator assetutil --info "$APPEX/Assets.car" | grep '"Name"'
  ```

- 重装后日志里不再有 `No image named`；主屏截图看组件内容。
- 注意：**旧构建残留在 appex 里的散帧图不会自动删**（Xcode 增量构建不清产物目录），
  它会和 `Assets.car` 共存 —— 不影响读取，别被它误导。

## 教训

「界面上什么都没有、日志只有一行」的组件问题，先按进程名抓 os_log；原生扩展的
失败大多不会冒到 JS 层。
