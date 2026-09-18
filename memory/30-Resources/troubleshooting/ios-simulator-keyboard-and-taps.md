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

## 5. 多设备 / 窗口标定 / 点击（2026-09-17 补充）

**多设备时不要用 `booted`**：两台以上设备 booted 时 `xcrun simctl io booted screenshot` 会选错设备，
一律显式传 UDID。

**在临时设备上验证「登录前页面」**（已登录设备的 `Stack.Protected` 会把 `/sign-in` 拦掉）：

- 从任一台已装 dev 包的模拟器 `xcrun simctl install <新设备> <Luka.app>` 即可复用同一 dev build；
- ⚠️ **卸载重装不清 Keychain**：`simctl uninstall` 后重装仍是「已登录」（退出登录才走得到的屏依旧
  不可达），只有 `simctl erase <设备>` 是真正的全新状态；erase 会清设备偏好，`HardwareKeyboardLastSeen`
  等要重新写。

**软键盘偏好会被改回去**：`defaults write … HardwareKeyboardLastSeen -bool false` 之后设备一重启就可能
被 Simulator 重写成 1（实测）。**验证「键盘弹起时的布局」不要依赖软键盘链**：临时把屏幕里的状态钉死
（如 `const keyboardVisible = true; // TEMP`）→ Fast Refresh 截图 → 还原。不用跟焦点/软键盘较劲。

**点击前的窗口标定**（System Events 的窗口 `position` 会漂、不可信）：

1. `CGWindowListCopyWindowInfo` 拿 window id + bounds（`owner == "Simulator"`）；
2. `screencapture -x -l<id> win.png` 截单窗口（含阴影，2x 每边几十像素，扫描确定）；
3. 在窗口图里按**内容特征色**扫设备屏边界（如奶油底 #FDF9EE：横竖各扫一行/列取 min/max）；
4. 映射：`MacX = winX + (imgX − shadow)/2`，`scale = 内容宽(px) / 设备宽(px)`；
5. 点击前 `activate Simulator` + `AXRaise` 目标窗口 —— 多个 Simulator 窗口默认会互相重叠，
   落在别的窗口上的点击会静默作用到那台设备上。

## 6. 点按「全落空」＝坐标系过期；时机敏感的交互要临时放宽（2026-09-17 补充）

**症状**：CGEvent 注入（`tap.swift`）连点 6 个洞位，App 里 0 命中；`System Events` 查
frontmost 是 Simulator、脚本也没报错 —— 很容易误判成「App 的 Pressable 有问题」。

**根因（两条叠在一起）**：
1. **窗口几何一变，手算的映射就全错**：本次还在用上一轮标定的常量
   `origin(190.5, 92)、scale(0.5196, 0.5277)`，而窗口已被移动/缩放过
   （现在 `(537,59) 622×856`）—— 旧坐标全部静默偏移，点在哪都不对。
2. **`swift /tmp/tap.swift` 每次调用要现场编译**，单次 1–3s —— 想连点 6 次要 10s+，
   而验证对象（打地鼠冒头）只停留 0.5–1.5s，等于每次都点空。

**修法**：
- 每次会话先**重新标定**：`System Events` 取窗口 `{position,size}` → 设备屏高宽比算内容高
  （`contentH = width * deviceH/deviceW`，余下即标题栏）→ `scale = width / deviceW_pt`
  → `MacX = winX + scale * x_pt`、`MacY = winY + titleBar + scale * y_pt`。**再用一个大目标
  验证一次**（如底部的 Next 按钮：点完看能不能换轮），别直接用坐标做实验。
- **`swiftc -O /tmp/tap.swift -o /tmp/tap` 编成二进制**，之后每次点按 ~0.1s，才谈得上连点。
- **时机敏感的交互（短停留、动画窗口）先临时放宽做验证**：把常量改成 `[6000,9000]` →
  Fast Refresh → 连点 → 截图确认机制（飞出/托盘/盖章）→ **立刻改回** `[500,1500]`。
  验证的是机制，不是时长；不要在静止截图里猜时长。

## 7. 多设备并行时：点按要 AXRaise + 截图要指名 UDID（2026-09-17 补充）

**症状**：CGEvent 点按「POSTED」成功、frontmost 也是 Simulator，App 毫无反应；下一轮盲点
甚至**误触了 App 的返回键**（弹出 dev 的 GO_BACK 错误层）。

**根因**：Simulator 里同时开着**两台设备窗口**（iPad Air 13 + iPhone 17，两个会话各用一台）。
`System Events` 的 frontmost 只保证「Simulator 这个 app」在最前，**不保证是哪一扇窗口** ——
点按落在另一台设备的窗口上，甚至落在目标 App 的别的控件上。

**修法**：
- 点按前把目标窗口抬起来：
  `tell application "System Events" to tell process "Simulator" to perform action "AXRaise" of window "iPad Air 13-inch (M4) – iOS 26.5"`
  （窗口名 = 设备名 + ` – ` + 系统版本，`screencapture -l`/`System Events` 都能拿到）。
- 截图/装包/深链一律**指名 UDID**，别用 `booted`（`xcrun simctl list devices booted` 看全）。
- 看不清点了哪里时先截一张看状态，别连续盲点（这次盲点顺手把 dev 错误层点出来了；清掉的办法
  是 `simctl terminate` + `launch`，比找 Dismiss 按钮的坐标快）。

## 关联

- [[rn-simulator-pixel-measurement]]（截屏量几何、deeplink 绕屏、Dev Menu FAB 关闭）
- [[mobile-keyboard-and-viewport]]（PWA 侧键盘让位：transform vs 压矮）

## 8. 拖拽/滑动手势：用 JXA 直接调 CGEvent，不必编译 Swift（2026-09-18 补充）

**需求**：验证 Luka Discover 刮刮乐的「按下 → 连续移动 → 抬起」与卡组上滑的仲裁，
不是单次 tap。本机 `cliclick` 没装、`python3` 无 pyobjc，但 **osascript 的 JXA 可以直接
`ObjC.import("CoreGraphics")`**，免编译、免安装：

```js
// /tmp/drag.js，`osascript -l JavaScript /tmp/drag.js`（先 activate Simulator）
ObjC.import("CoreGraphics");
function post(type, x, y) {
  $.CGEventPost($.kCGHIDEventTap, $.CGEventCreateMouseEvent($(), type, $.CGPointMake(x, y), $.kCGMouseButtonLeft));
}
const origin = $.CGEventGetLocation($.CGEventCreate($())); // 结束移回原位
post($.kCGEventLeftMouseDown, x0, y0);
for (let i = 1; i <= 24; i += 1) {
  post($.kCGEventLeftMouseDragged, x0 + ((x1 - x0) * i) / 24, y0 + ((y1 - y0) * i) / 24);
  delay(0.012);
}
post($.kCGEventLeftMouseUp, x1, y1);
delay(0.3);
post($.kCGEventMouseMoved, origin.x, origin.y);
```

- 每步 `delay(0.01~0.015)`：太快 Pan 手势收不到完整轨迹（同第 2 节 tap 要 30ms 的道理）。
- 坐标映射用「窗口 `{position,size}` + 截图像素 / 3 的比例」直接换算即可打到卡片这种大目标；
  小目标仍按第 5 节标定。窗口会被移动/缩放，**每次会话重新取**。
- 验证有业务副作用的动作（向上滑 = pass 扣额度）要谨慎：改拖**反方向**（向下/水平）验证
  「不误触划卡」与仲裁，不拿真数据做实验。
