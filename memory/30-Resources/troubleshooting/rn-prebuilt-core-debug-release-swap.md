---
title: RN 预编译 core 变体被调包：Debug 构建链到 Release 的 React.framework
date: 2026-09-20
tags: [ios, react-native, cocoapods, expo, 构建, troubleshooting, sitin-rn]
---

出处：sitin-rn `apps/luka`，2026-09-20（Xcode 26.6 / RN 0.86 / Expo SDK 57，模拟器 Debug 构建）。

# 现象

`expo run:ios` 链接 `Luka.debug.dylib` 失败：

```text
ld: warning: Could not find or use auto-linked framework 'CoreAudioTypes' ...
ld: warning: Could not find or use auto-linked framework 'UIUtilities': framework 'UIUtilities' not found
Undefined symbols for architecture arm64:
  "_OBJC_CLASS_$_RCTPackagerConnection" / "_OBJC_CLASS_$_RCTReconnectingWebSocket"
  "facebook::react::Sealable::Sealable()" / "~Sealable()" / "ensureUnsealed()"
  "facebook::react::DebugStringConvertible::getDebug*()"
  "facebook::react::ShadowNode::getDebug*()" / typeinfo / VTT for BaseViewProps、YogaStylableProps、Props
ld: symbol(s) not found for architecture arm64
```

两条 `auto-linked framework` 是 **warning 不是 error**（`UIUtilities` 在 SDK 里是头文件-only 的
SubFramework，Xcode 26.5+ 的 UIKit 会 import 它，ld 找不到可链接产物但不影响链接结果）；
真正挂住的是那批 **dev-only / Debug 变体才导出**的符号。

# 根因

RN 0.86 的预编译 core 有两份 Maven 产物（`ReactNativeCore-artifacts/reactnative-core-<ver>-{debug,release}.tar.gz`），
`pod install` 默认装 **debug** 变体；`React-Core-prebuilt` 的脚本阶段
`[RNCore] Replace React Native Core for the right configuration` 会在构建时按当前 configuration
调 `replace-rncore-version.js` 把 `Pods/React-Core-prebuilt/React.xcframework` 换成对应变体。

它判断「要不要换」只看一个放在 **Pods 目录里的 marker**：

```text
apps/<app>/ios/Pods/React-Core-prebuilt/.last_build_configuration   # 内容 "Debug" / "Release"
# 无 marker 且当前是 Debug → 脚本按注释假设「装的已经是 Debug」→ 直接跳过替换
```

于是本地按这个顺序就会坏：

1. 模拟器 Debug 构建正常（framework = debug，无 marker）；
2. `pnpm --filter luka build-ipa`（`xcodebuild -configuration Release` archive）→ 脚本把 framework
   换成 **release** 变体，写 marker `Release`；
3. 之后任意一次 `pod install`（如加了个新 pod、`expo run:ios` 自动触发）→ **marker 被清掉**，
   而 framework 里的 release 内容原样保留；
4. 再跑 Debug 构建 → 无 marker → 跳过替换 → 拿 release core 链 Debug 包 → 上面那批
   dev-only 符号全部 undefined。

即：**状态判断依据（marker）放在 CocoaPods 会清理的目录里**，是上游 RN 的设计坑。

# 判断口径（30 秒）

```sh
cd apps/<app>/ios/Pods
cat React-Core-prebuilt/.last_build_configuration            # 不存在 = 状态可疑
FW=React-Core-prebuilt/React.xcframework/ios-arm64_x86_64-simulator/React.framework/React
lipo -thin arm64 -output /tmp/rncore-arm64 "$FW"
xcrun dyld_info -exports /tmp/rncore-arm64 | grep -c RCTPackagerConnection
#   >0 = debug 变体；0 = release 变体（当前构建是 Debug 就是错的）
```

要更确定可与两份 tarball 解出的同片 binary 比 `shasum`。

# 修法

不用重新 `pod install`（它只会再次抹掉 marker），直接 seed marker 逼脚本替换回 Debug：

```sh
cd apps/<app>/ios/Pods
printf 'Release' > React-Core-prebuilt/.last_build_configuration
node "$(node --print "require.resolve('react-native/scripts/replace-rncore-version.js')")" \
  -c Debug -r <react-native 版本，如 0.86.0> -p "$PWD"
# 脚本跑完 marker 自动写成 Debug，framework 换成 debug 变体
```

然后直接重新构建即可（`xcodebuild ... build` 或 `pnpm <app>:ios -- ...`），
已验证 `** BUILD SUCCEEDED **`。

# 预防

- 同一台机器本地出完 Release 包（`build-ipa`）后，紧接着跑模拟器 Debug 前，先看一眼
  `Pods/React-Core-prebuilt/.last_build_configuration` 是否存在、是不是 `Debug`。
- 更稳的做法是在模拟器构建前置脚本（本仓库 `scripts/ensure-simulator-pods.sh` 这类地方）
  用上文的 `dyld_info` 符号检查兜底：marker 缺失且 framework 是 release 时强制换回 Debug。
- 修完不用提交任何东西 —— 改动只落在 gitignore 的 `ios/` 生成目录里。

# 相关

- [rn-prebuilt-core-dyld.md](rn-prebuilt-core-dyld.md) —— 另一类预编译 core 问题：
  产物存在性检查失败静默回落源码编译，与预编译 Expo 模块混搭导致 dyld 崩溃。
- 上游脚本：`node_modules/react-native/scripts/replace-rncore-version.js`、
  `scripts/cocoapods/rncore.rb`。

## 追加（2026-09-23）：marker 修完仍要防「Expo 预编译模块 × core 变体」混搭

同一天模拟器重建：先按上文 seed marker + `replace-rncore-version.js` 换回 Debug，
**链接通过**，但 app 一启动就 SIGBUS 崩：

```text
EXC_BAD_ACCESS / KERN_PROTECTION_FAILURE
React           facebook::react::YogaStylableProps::YogaStylableProps(...)
React           facebook::react::BaseViewProps::BaseViewProps(...)
ExpoModulesCore expo::ExpoViewProps::ExpoViewProps(...)
ExpoModulesCore RawPropsParser::prepare<expo::ExpoViewProps>()
```

即 debug 的 React core 与**按 release 时期 stage 出来的 Expo 预编译模块**（ExpoModulesCore 等）
ABI 混搭 —— `replace-rncore-version.js` 只换 React core / ReactNativeDependencies 两个
xcframework，不碰 Expo 的预编译 pod。

**修法（一次到位）：**

```sh
rm -rf apps/<app>/ios/Pods
cd apps/<app> && pod install --project-directory=ios   # 缓存命中，会按 debug 重新 stage Expo 模块
# 然后正常构建（expo run:ios）
```

**判断口径：** 崩溃栈同时出现 `ExpoViewProps`/`ExpoViewShadowNode` 构造与
`YogaStylableProps`，且刚经历过 Release→Debug 的方向切换，就按这个处理。
**教训：换 core 变体不能只换 core，Pods 里其它预编译产物要一起重来 —— 删 Pods 重装最省心。**

同场加映（另一个坑）：同事提交引入新的原生模块（`expo-web-browser`）后，**旧模拟器包
重拉 bundle 会红屏 `Cannot find native module 'ExpoWebBrowser'`** —— 必须 `pod install` +
重构建，错误信息与 pod 无关时先查最近提交有没有加依赖。
