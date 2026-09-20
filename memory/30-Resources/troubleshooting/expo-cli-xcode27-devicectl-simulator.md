---
title: Xcode 27 下 expo run:ios 装不进模拟器：devicectl 把模拟器当物理设备（CoreDeviceError 1001）
date: 2026-09-20
tags: [ios, xcode, expo, react-native, 构建, troubleshooting, sitin-rn]
---

出处：sitin-rn `apps/luka`，2026-09-20（Xcode 27.0 / iOS 27 SDK，macOS 模拟器）。

# 现象

构建成功、`.app` 已产出，但装模拟器失败：

```text
› Installing .../DerivedData/Luka-.../Build/Products/Debug-iphonesimulator/Luka.app
✖ Connecting to: iPhone X (11 Pro eq)
Error: ERROR: The capability "Install Application" is not supported by this device.
       (com.apple.dt.CoreDeviceError error 1001 (0x3E9))
  at ChildProcess.<anonymous> (.../@expo/cli/build/src/start/platforms/ios/devicectl.js:291:29)
```

注意：报错栈指向 **devicectl**（物理设备安装通道），但目标是模拟器 —— 这就是线索本身。

# 根因

Xcode 27 起 `xcrun devicectl list devices` **会把模拟器也列出来**（`Reality = simulated`）：

```text
iPhone 18 Pro          D0F2CAF7-... (UDID)   shutdown   iPhone 18 Pro (iPhone19,2)   simulated
iPhone X (11 Pro eq)   7DE771A0-... (UDID)   shutdown   iPhone 11 Pro (iPhone12,3)   simulated
```

@expo/cli **< 57.0.14** 的 `getConnectedDevicesAsync()` 把 devicectl 输出无条件映射为
`deviceType: 'device'`（= 物理设备），再和 `simctl` 的列表按 UDID `uniqBy` 合并，
**devicectl 的条目排在前面** → 模拟器被物理条目遮蔽。`--device <名字>` 解析到它后：

1. `isSimulatorDevice()` 判 false（`deviceType: 'device'` 不以 `com.apple.CoreSimulator.SimDeviceType.` 开头）；
2. 走物理设备安装（usbmuxd）→ 找不到设备（`APPLE_DEVICE_USBMUXD`）；
3. 回落 devicectl 通道 → 对模拟器报 `Capability "Install Application" is not supported`。

模拟器本身没问题（手动 `xcrun simctl install` 同一 `.app` 直接成功）。

# 修法（已落地本仓库）

`pnpm-workspace.yaml` 的 `overrides` 里钉 `'@expo/cli': 57.0.26`（expo 声明 `^57.0.9`，范围内）。
57.0.14 起 `getConnectedAppleDevicesAsync()` 会过滤 `hardwareProperties.reality === 'simulated'`，
并接受 devicectl 的 jsonVersion 5（Xcode 27 输出）。

```sh
pnpm install
node -e "console.log(require('./node_modules/@expo/cli/package.json').version)"   # 57.0.26
```

验证：`resolveDeviceAsync('iPhone X (11 Pro eq)')` 返回带 `deviceTypeIdentifier` 的 simctl 条目
（`isSimulatorDevice` → true）；`pnpm luka:ios -- --device 'iPhone X (11 Pro eq)'` 走到
`Opening on iPhone X (11 Pro eq)`，exit 0。

# 绕过（不改仓库）

`--device` 一旦传名字/UDID 就会走上面的合并逻辑（物理条目仍会遮蔽）。绕开办法是
**先手动启动目标模拟器，再不带 `--device` 跑** `expo run:ios`：

```sh
xcrun simctl boot <udid>
pnpm <app>:ios            # 不带 --device；Expo 用 getBestBootedSimulatorAsync（纯 simctl 路径）
```

适用：暂时不想动 lockfile，或上游版本还不能升。

# 相关

- 同一天 Xcode 26 → 27 升级引出的另一个坑：`xcode26-swiftuicore-sim-link.md`
  （SwiftUICore stub 过期，undefined `_preferenceValuesEqual`）。
- 上游修复：@expo/cli 57.0.14（过滤 simulated）、57.0.26（当前钉版）。
