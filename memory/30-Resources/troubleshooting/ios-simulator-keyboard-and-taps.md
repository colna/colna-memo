---
title: iOS 模拟器：软键盘不弹 / 点不中 TextInput / 录屏被占
date: 2026-09-16
tags: [troubleshooting, ios, simulator, keyboard, automation]
---

# iOS 模拟器：软键盘不弹 / 点不中 TextInput / 录屏被占

来源：sitin-rn `apps/luka` 聊天输入条「键盘跟手」验证（commit `087d00e3`，2026-09-16）。
三个坑都会让人误判成「App 代码有问题」，实际都是模拟器/宿主环境。

## 1. focus 了也不弹软键盘：设备记得「见过硬件键盘」

**症状**：点 TextInput 有光标/编辑菜单，但软键盘不出现；`.devx` 里 autoFocus 的字段
同样只有光标没有键盘。

**根因**：模拟器设备偏好 `com.apple.keyboard.preferences` 的 `HardwareKeyboardLastSeen = 1`
—— iOS 认为接着硬件键盘，就永久不弹软键盘（`ConnectHardwareKeyboard` 关掉也不改这条记忆）。

**修法**（二选一，可叠加）：

```sh
xcrun simctl spawn booted defaults write com.apple.keyboard.preferences HardwareKeyboardLastSeen -bool false
xcrun simctl spawn booted defaults read  com.apple.keyboard.preferences | grep -i hardware
```

- 立刻见效的是 Simulator 菜单：**I/O → Keyboard → Toggle Software Keyboard**（对一个已 focus
  的字段显式弹出；没有 focus 的字段时按了没反应）。
- 想让它自动弹：置 `false` 后重启设备（`simctl shutdown` + `boot`）。
- 菜单项可用 AXPress 远程点：`tell application "System Events" to tell process "Simulator" to
  perform action "AXPress" of menu item "Toggle Software Keyboard" of menu 1 of menu item
  "Keyboard" of menu 1 of menu bar item "I/O" of menu bar 1`（先 `activate` Simulator，
  设备菜单栏不在前台时 AXPress 静默无效）。

> 顺带：`I/O → Keyboard → Connect Hardware Keyboard` 的勾选状态读 `AXMenuItemMarkChar`
> （`✓` = 开）。它只管「宿主键盘是否打进设备」，管不了上面那条 `LastSeen` 记忆。

## 2. System Events 的 click 点不中 TextInput

**症状**：`osascript … click at {x,y}` 能点按钮、Tab、列表行，**点 TextInput 不聚焦**
（无光标、无键盘）；对同一坐标的 `+` 按钮却有效，容易怀疑是 App 的 touch 被挡。

**根因**：瞬时 click（按下即抬起）不足以让 UIKit 的 text input 进入编辑态；实际是一次
「点得太快」。而且 90ms 的按住会触发**长按编辑菜单**（粘贴/自动填充），又是另一种误判。

**修法**：用 CGEvent 精确控制按住时长（30ms 左右最像 tap）：

```swift
// /tmp/tap.swift，`swift /tmp/tap.swift 428 843`
import CoreGraphics
import Foundation
let pos = CGPoint(x: Double(CommandLine.arguments[1])!, y: Double(CommandLine.arguments[2])!)
CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: pos, mouseButton: .left)?.post(tap: .cghidEventTap)
usleep(30000)
CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: pos, mouseButton: .left)?.post(tap: .cghidEventTap)
```

**坐标映射**（Simulator 窗口 → 设备 pt）：不要手算窗口位置/scale，直接标定：

1. `System Events` 取窗口 `{position, size}`（本次 `230,57,396,852`）。
2. `screencapture -x -R<position,size> /tmp/win.png`（输出是 2x 像素）。
3. 在截图里找设备屏的四条边（在屏幕中部横向/纵向扫一行，黑边 → 内容即边界），
   得到设备屏在窗口内的偏移与缩放。本次：设备屏左上在窗口内 (23.5, 78) Mac pt、
   scale≈0.8667（与 Simulator 自动记录的 0.8681 一致）。
4. 映射式：`MacX = 230 + 23.5 + 0.868·x_pt`、`MacY = 57 + 78 + 0.8667·y_pt`。
   底部（tab bar / 输入条）用同一式子命中，多次点击验证过。

## 3. `simctl io recordVideo` 报 “Host recording is already in progress”

**症状**：`xcrun simctl io booted recordVideo --codec=h264 --force /tmp/x.mov` 一直报
`NSPOSIXErrorDomain Code=16`，但 `ps` 里没有任何 simctl/ffmpeg 进程。疑似 CoreSimulator
服务里的残留锁（也可能是并行的另一会话仍在录）。

**绕法**：录宿主屏幕上的 Simulator 窗口区域，完全绕开 simctl：

```sh
screencapture -x -v -R230,57,396,852 /tmp/out.mp4   # 120fps、含窗口边框
kill -INT <pid>                                     # 停止
# 窗口内设备屏边界标定后：device_px = (capture_px - 偏移) / 0.5788
```

`-x` 免声音，`-v` 视频模式。产物是 mp4（扩展名随意）。

## 4. 逐帧量「跟手」的方法（本项目用过的）

1. 录 120fps，`ffmpeg` 抽两列 raw（`crop=16:1704:x:0`；**yuv420p 不能 crop 宽 1**，用 16 取中间列）。
2. 在**键盘**里找一个色标（如蓝色 send 键）做「键盘位置」，在**输入条**里找色标（cocoa 发送键）做「输入条位置」。
3. 逐帧输出两者 top 的差：上升全程差单调收敛、无先涨后落的尖峰 = 同步；有滞后尖峰 = 输入条掉队。
4. 别用 h264 里的**颜色阈值**直接认色：窄列受 chroma 子采样+压缩污染（cocoa 104,68,49 变 165,157,139），
   用轮廓/亮度或改用无损截图标定阈值。

## 关联

- [[rn-simulator-pixel-measurement]]（截屏量几何、deeplink 绕屏、Dev Menu FAB 关闭）
- [[mobile-keyboard-and-viewport]]（PWA 侧键盘让位：transform vs 压矮）
