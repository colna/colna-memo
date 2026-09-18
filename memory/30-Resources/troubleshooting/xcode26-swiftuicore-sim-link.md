---
title: Xcode 26 模拟器构建：cannot link directly with 'SwiftUICore'
date: 2026-09-18
tags: [ios, xcode, cocoapods, react-native, expo, 构建, troubleshooting]
---

# 现象

Xcode 26（26.5/26.6，iOS 26.x 模拟器 SDK）下，任何链接带 SwiftUI 代码的 pod（TUICallKit、
SwiftUIIntrospect、ExpoUI、expo-glass-effect…）的 iOS 模拟器构建失败：

```
ld: cannot link directly with 'SwiftUICore' because product being built is not an allowed client of it
```

（有时以 warning 出现：`Cannot parse or use implicit file '.../SwiftUICore.tbd'`，并伴随大量
无关 `Undefined symbols`——那是 ld 解析受限 tbd 失败后的连锁反应。）

真机构建（iPhoneOS SDK）不受影响；Xcode 16 上不存在该问题（当时 weak 方案有效）。

# 根因（实测）

- **Swift 5 语言模式**下，每个 `import SwiftUI` 的编译单元都会 strong autolink
  **`-framework SwiftUI` + `-framework SwiftUICore`**；Swift 6 模式只 autolink SwiftUI。
  验证：
  ```sh
  printf 'import SwiftUI\nstruct V: View { var body: some View { Text("x") } }\n' > s.swift
  xcrun swiftc -sdk "$(xcrun --sdk iphonesimulator --show-sdk-path)" \
    -target arm64-apple-ios16.4-simulator -swift-version 5 -parse-as-library -c s.swift -o s5.o
  otool -l s5.o | grep -A 3 LC_LINKER_OPTION   # 会看到 SwiftUICore
  ```
- iOS 26 模拟器 SDK 的 `SwiftUICore.tbd` 带 `allowable-clients:` 白名单（只有 SwiftUI、
  UIKitCore 等），普通 app 直接链接即被拒。
- 旧方案 `-weak_framework SwiftUICore`（Xcode 16 有效）在 Xcode 26 **失效**，且**反向引爆**：
  对没有 SwiftUICore 引用的 target（NSE、HomeWidget），这个 flag 会让 ld 去加载受限 tbd，
  报同一个错。`-lazy_framework` 同样无效（实测）。
- `-Xfrontend -disable-autolink-framework -Xfrontend SwiftUICore` 能修**本仓库能重编的**
  Swift 单元，但修不了 Swift-5 模式**预编译**的产物（Expo precompiled、React-Core-prebuilt
  等）；而且禁掉 autolink 后 app 链接会缺一批符号（实测连环 undefined）。

# 解法（已落地 rn-trtc 0.13.1）

在 Podfile 的 `post_install`（`@heyhru/rn-trtc` 的 `app.plugin.js` 注入）：

1. 从 `xcrun --sdk iphonesimulator --show-sdk-path` 取真实 `SwiftUICore.tbd`；
2. 拷一份、用正则删掉 `^allowable-clients:` 段（`sub(/^allowable-clients:.*?(?=^\S)/m, '')`），
   写入 `ios/SwiftUICoreStub/SwiftUICore.framework/SwiftUICore.tbd`（生成物，不入库）；
3. 给 App 与扩展 target 的 **`FRAMEWORK_SEARCH_PATHS[sdk=iphonesimulator*]`** 加上该目录
   （条件键保证真机构建不受影响）；
4. 顺手清掉遗留的 `-weak_framework SwiftUICore`（老工程升级时会残留）。

符号表保持完整（Widget 会直接引用 SwiftUICore 符号，空 stub 不够），客户端限制消失；
运行时加载的仍是系统 `/System/Library/Frameworks/SwiftUICore.framework`（同 install-name）。

# 验证

```sh
cd apps/<app>/ios && env -u HEYHRU_BYTEPLUS_NATIVE pod install
grep -c SwiftUICoreStub Luka.xcodeproj/project.pbxproj   # 6 = app/扩展 × Debug/Release
xcodebuild -workspace <App>.xcworkspace -scheme <App> -configuration Debug \
  -destination 'platform=iOS Simulator,id=<udid>' build | tail -3   # ** BUILD SUCCEEDED **
```

# 弯路记录

- `-weak_framework` / `-lazy_framework`：压不住 autolink；无引用 target 反而报错。
- 全局禁用 autolink：预编译产物修不了 + 连锁缺符号。
- 固定空 stub：app 主 target 能过，Widget/扩展会因引用 SwiftUICore 符号而 undefined ——
  stub 必须带真实符号表。
- `-swift-version 6` 能天然避开，但不现实（第三方源码过不了 Swift 6 检查）。

# 相关

- 上游旧结论（Xcode 16）与归属：`apps/iris/docs/05-native-capabilities.md` §1.1.a。
- 实现与发布记录：`packages/rn-trtc/CHANGELOG.md` 0.13.1。
