---
title: sitin-pwa「Authorize accounts」授权抽屉反复弹
date: 2026-08-17
tags: [troubleshooting, sitin-next, app-pwa, modal]
---

# sitin-pwa 授权小抽屉反复弹出

## 坑
`showSocialAuthDrawer`(「Authorize accounts」小抽屉)在某社媒 expired 时反复弹出,
用户点「Maybe later」关掉后立刻又弹。

## 根因(两条叠加)
1. **触发源反复 re-fire**:`App.tsx` 调 `initUser` 的 effect 依赖含
   `[isReady, userState, userInfo?.userId, ...]`,其中任一变化(在线/工作态切换、用户信息刷新)
   → 重跑 `initUser` → `initInsTask` → `checkSocialProxyAbnormal` → 结尾
   `showSocialAuthDrawerFromStore()`。而 expired(`authByPlatform[p]===true && abnormalByPlatform[p]===true`)
   在重连成功前不会变,所以每次 re-init 都满足弹出条件。
2. **抽屉无任何抑制/去重**:`showSocialAuthDrawer` 直接 `modalStore.open(MODAL_ID,...)`,
   同 id `open` 会覆盖内容并**重播 slide-in 动画**;且没有「用户已关闭本次会话就别再自动弹」的记忆
   (对比新写的 `SocialAuthTaskDrawer` 有 `collapsedTaskId`,老抽屉漏了)。

## 修法(A 会话级抑制 + B 已打开去重,commit 2950e9e06)
- 模块级 `let drawerDismissed=false`:
  - `showSocialAuthDrawer` open 时登记 `onClose: () => drawerDismissed=true`(「Maybe later」/点遮罩都记)。
  - `showSocialAuthDrawerFromStore` 开头:`if (!opts?.force && drawerDismissed) return`。
  - `dismissSocialAuthDrawer`(授权成功主动关):close 后 `drawerDismissed=false`(让剩余 expired 平台重连后仍能弹)。
  - `resetSocialAuthDrawerDismissed()` 导出,在 `useUserInit.cleanUser` 切/登出用户时调,防跨用户继承抑制。
- 已打开去重:`showSocialAuthDrawer` 开头 `if (modals.some(m=>m.id===MODAL_ID)) return`。
- 设置页「Link Social Media」用 `force:true`,**绕过**抑制照常弹。

## 通用经验
「系统按状态自动弹的弹窗」凡是可被多个生命周期事件重复触发的(init/前台/订阅),
必须配「已打开去重」+「用户已关闭本次会话抑制」,并在切换用户/成功处理时重置。
参考同仓 `SocialAuthTaskDrawer` 的 `collapsedTaskId` 模式。
