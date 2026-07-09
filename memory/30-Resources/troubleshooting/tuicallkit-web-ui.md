---
title: TUICallKit Web UI 定制能力边界(为什么 sitin PWA 通话页自研)
date: 2026-07-08
tags: [troubleshooting, sitin-next, app-pwa, tuicall, trtc, 通话]
---

# TUICallKit Web:UI 能不能只留挂断按钮?

**结论:不能(不推荐)。Web 端 UIKit 不开源、无按钮级配置。sitin PWA 通话页应继续自研。**

## 坑 / 问题

看到腾讯 TUICallKit 的通话界面(黑底 + 头像 + `Connecting...` + 「翻转 / 挂断 / 摄像头已开」三按钮 + 右上画中画),
想直接拿来做**主动外呼等待页**,只保留「挂断」。

## 根因(2026-07-08 查证)

1. **Web 端 UIKit 源码不开源。** MIT 仓库 `Tencent-RTC/TUICallKit` 里 `Web/` 目录只有 **demo**
   (`basic-react` / `basic-vue2.6` / `basic-vue2.7` / `basic-vue3`),demo 通过 npm 引入预编译 UI 包
   (`@trtc/calls-uikit-react@~4.4.9`;旧名 `@tencentcloud/call-uikit-react`)。
   真正开源的是 iOS / Android / Flutter / ReactNative / MiniProgram 的**原生**源码。
2. **无按钮级 props。** 官方文档(trtc.io/document/59842)里 `<TUICallKit>` 只暴露
   `allowedMinimized` / `allowedFullScreen`。没有隐藏「翻转 / 摄像头」的开关。
3. 硬做只剩三条烂路:patch npm 包 / 靠内部 class 名 CSS 强隐 / fork 打包产物 —— **随版本升级即碎**。

## 修法 / 正确姿势

**自研 UI,只用引擎。** sitin-next `app-pwa` 已声明:

- `tuicall-engine-webrtc@^3.1.7` —— **纯逻辑引擎,无 UI**,用在 `src/services/webCallManager.tsx`
- 通话 UI 全部自研:`pages/VideoCall`、`pages/AudioCall`、`pages/MockCall`、`components/OutgoingCallModal.tsx`
- 全仓 `grep call-uikit / TUICallKit` 零命中;`node_modules/@tencentcloud` 下无 `call-uikit-*`

外呼**等待页**尤其不该用 UIKit:此时还没接通、没有远端流,不需要翻转/开关摄像头,
只需要「头像 + 名字 + Connecting… + 挂断」。用 UIKit 等于先引大包、再想办法藏它多余的按钮。

## 关键业务约束(换 UIKit 会丢)

外呼的「挂断/取消」不是纯 UI 动作:

- `pages/VideoCall/index.tsx` 有「页面打开后 5 秒内不允许挂断」的倒计时逻辑。
- 线上 `finishCallOrder` 的 `reasonType` 区分 `MALE_CANCEL_TIMEOUT_5S` / `WITHIN_5S`
  (男方振铃等待是否超 5s,决定计费与统计),**由消费端随请求上报**。

若换成 TUICallKit 自带的挂断按钮,这条业务钩子要另找地方接回来。

## 相关

- 主动外呼实现见 [[../../50-Daily/2026-07-07]](PR #540:`OutgoingCallModal` + `useWebCall.startOutgoingCall`,
  接通前走独立 `CALL_OUTGOING_ENDED` 路径,避免误触发女方收益结算)
- `OutgoingCallModal.tsx` 截至 2026-07-08 仍是 64 行**占位实现**(注释:「设计稿待定,最小占位 UI:头像/名称/状态 + Cancel」)

## 参考

- https://github.com/Tencent-RTC/TUICallKit (MIT,Web/ 仅 demo)
- https://trtc.io/document/59842 (TUICallKit Web 文档)
