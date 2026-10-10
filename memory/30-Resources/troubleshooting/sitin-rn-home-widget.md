---
title: sitin-rn 桌面小组件（rn-home-widget）：模式、坑与验收
date: 2026-10-09
tags: [troubleshooting, sitin-rn, widget, ios, expo, rn-home-widget, sofia]
---

# 背景

sitin-rn 的桌面组件能力在 `packages/rn-home-widget`（iOS WidgetKit / Android AppWidget），
各 app 线（luka / beth / rocco / nugget / sofia…）分别消费。2026-10-09 给 sofia 新增
**消息模式（0.6.0）**：收消息 → 名称闪两下 → 打字机出正文。以下是与包打交道时的可复用结论。

# 模式与机制

- **frames 模式**（beth 线 0.5.0 最新）：build 时把 `widgets/frames/frame-NN.png` 打进
  asset catalog，iOS 一条 timeline entry 一帧、系统预渲染按墙钟翻页（播完停末帧）；Android 用
  `ViewFlipper` 真循环。**帧图必须进 asset catalog**，散文件 `Image(name)` 静默找不到。
- **states 模式**（luka 0.4.0）：读 App Group 快照，Swift 自行从 `unreadCount`/`lastInteractionAt`
  推导状态 + 画 app 给的四句文案。
- **messages 模式**（sofia 0.6.0，`messages` 选项）：读快照最后一条消息，9 个透明度关键帧
  演「闪两下」，正文每步一条 entry（单字，>90 条多字一步）+ 光标；字符按 Swift `Character` 切。
  与 `links`/`palette` **互斥**；**不读帧图、不生成 Android**（AppWidget 要另写文本模板）。
- 两端共同硬边界：组件不是进程，**没有渲染循环**；iOS 的 reload 是按天预算的*请求*，
  动画只在系统重建时间线时播一次。别承诺「一直动」。

# 新增/修改时的必做

1. **`expo prebuild --clean`**：生成器对已存在的 widget target 幂等（同名直接 return），
   普通 prebuild 不会重写 Resources 阶段 → 换模板/换帧数不 clean 等于没换。
2. **App Group 与扩展 id 都是身份**，无默认值：`appGroup: group.<bundleId>`、
   `bundleIdSuffix`。非生产档所有 app 共用主包名（`com.presence.gracechat`）时，
   扩展 id 要带 app 前缀（sofia 用 `sofia.widget`、通知扩展先例 `sofia.notiservice`），
   否则装两个 house 包互相覆盖；**App Group 也是共用的 → 快照键必须带 app 命名空间**
   （sofia 用 `sofia.widget.message.v1`，不要裸 `widget.snapshot`）。
3. 真机/出包要 Apple 后台注册 App Group 并给两个 App ID 开 capability（模拟器不用）；
   换模板不能 OTA，必须重出基包。
4. 改 Swift 模板的正确性验证：`swiftc -typecheck -parse-as-library -sdk $(xcrun --sdk
   iphonesimulator --show-sdk-path) -target arm64-apple-ios17.0-simulator Foo.swift`
   —— 不用起模拟器就能查出 API/拼接错误（实战查过 message 模板全变体零警告）。
5. 生成的 Swift 有**冻结基线测试**（`tests/ios-swift.test.ts`）：不传新参数时输出必须逐字
   不变，新能力一律长成独立模板而不是在旧模板上做条件拼接。

# 无图形 Simulator 时的验收（2026-10-09 实测路径）

1. `expo prebuild --clean` → 检查 `ios/HomeWidget/`（Swift/plist/entitlements/catalog）、
   主 App `Info.plist` 的 `AppGroupIdentifiers`、pbxproj target、Podfile ccache 修复；
2. `expo run:ios`（**`--no-bundler` 可能被 pnpm 双层透传吃掉**，注意日志里有没有起 Metro；
   也可能直接复用别的会话已在跑 8081 的 Metro）→ 检查 `PlugIns/HomeWidget.appex` 与
   bundle id；
3. 触发链路：Metro CDP 求值 `expo.modules.HomeWidget.setItem/reload`（见
   [rn-metro-cdp-eval-headless.md](rn-metro-cdp-eval-headless.md)），宿主读
   `<AppGroup>/Library/Preferences/group.<id>.plist`；cfprefsd 有缓存，直接读文件可能滞后
   一次写入，以 native 模块 `getItem` 读回为准；
4. 视觉（加组件到桌面看动画）**必须图形化模拟器/真机**：本机 Xcode 无 Simulator.app，
   `simctl` 无触摸注入、`openurl` 自定义 scheme 会弹确认框，做不了。App Group 里预写好
   快照，用户在图形模拟器上长按桌面加组件即可直接看到动画（加组件时必构建新时间线）。

# 本地构建两坑

- CocoaPods 报 `Unicode Normalization not appropriate for ASCII-8BIT` = 缺 locale：
  `export LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8`（sofia 的 `ios` 脚本内置，裸跑
  `npx expo prebuild` 没有）。
- Expo 的 config 加载器**不能 import TS 源文件**到 `app.config.ts`（需要常量一致时：写
  字面量 + 一条单测读 `app.config.ts` 文本钉住与常量相同，sofia 已这么做）。

# 内容取向（2026-10-10 定）

**组件不是消息流/通知的替代。** 消息事件归系统推送与 App 内通知；组件再放一份既滞后
（reload 预算、App 没运行不更新）又把私聊内容暴露在桌面。适合组件的是**状态型内容**：
每天变化、App 不开也有意义、不携带私密正文。sofia 的结论 = 「今日推荐 + 开场白」，
并顺手把两个表现开关做成快照字段（同样不必重出基包）：

- `charIntervalMs`：打字节奏（毫秒/字，夹 `[30, 1000]`）。75ms 在桌面会明显被合并成
  「一顿一顿」，130ms 是稳的默认。
- `flash`：名称是否「闪两下」。闪是通知语汇，语义不是通知的内容（AI 在写字）传 `false`。

锁「当天内容」的一贯做法：把选中的 `{date, pick...}` 存 app 侧（sofia 用
`lib/storage` 的 `sofia.widget.daily.v1`），当天不再重选；用户清了卡就按缓存补发、
不重新选（否则划卡会让组件跳人）。
