---
title: RN 输入框在滚动区、按钮在滚动区外：键盘起时按钮盖住输入框
date: 2026-09-21
tags: [troubleshooting, react-native, keyboard, sitin-rn, luka]
---

# RN 输入框在滚动区、按钮在滚动区外：键盘起时按钮盖住输入框

来源：sitin-rn `apps/luka/src/app/settings/delete.tsx`（删号确认屏），2026-09-21 用户录屏报「按钮压在输入框上」。同一失败类此前在 koda 举报页出现过（memo 2026-09-03 R00272，PR #375）。

## 现象

键盘弹起、焦点在确认框（滚动区**末尾**），屏底部钉着的操作栏（`Delete my account` / `Keep my nest`）正好压住输入框下半截，输入框下面那行提醒完全看不到。

## 根因（三层，缺一不可）

1. `KeyboardAvoidingView(behavior="padding")` 只把滚动区**视口压矮**，不会让聚焦输入框滚进可视区。iOS 的 UIKit 自动滚只把输入框滚到「键盘上沿」，**不知道固定栏的高度**，于是输入框落在栏底下。
2. 栏的底部 padding 用 `useBottomInset()`（非键盘感知）：键盘起时仍留 `max(insets.bottom, 16)`（刘海机 34pt），把栏额外抬高 26pt，等于再从滚动区抢走 26pt。键盘态该用的是 `useKeyboardAwareBottomInset()` → `KEYBOARD_GAP = 8`。
3. focus 时没有任何「把输入框滚出来」的动作，滚动位置就停在 UIKit 默认滚到的位置。

## 修法（luka 删号屏，改动面最小的一种）

1. 栏改 `useKeyboardAwareBottomInset()` —— 与 `components/post/post-action-bar.tsx` 同款（该屏输入框在栏**里面**，所以只需这一条）。
2. ScrollView 加 ref，确认框 `onFocus` → 等键盘动画走完（300ms，键盘动画约 250ms）再 `scrollToEnd({ animated: true })`：内容末尾是「输入框 → 提醒行 → `pb-6`」，滚到底就自然停在栏上方。

## 选型判据

- 输入框**在固定栏里**（luka 帖子评论条）→ 只修栏的键盘态 padding。
- 输入框在滚动区、栏在滚动区外（本屏）→ padding + 把输入框滚出来，两条都要。
- 要「聚焦输入自动避开键盘」的原生滚法：`KeyboardAwareScrollView`（keyboard-controller），`bottomOffset` 预留固定栏高度（koda 举报页的结论，见 memo 2026-09-03）。本屏取 `scrollToEnd` 方案是因为结构已与 `post/[id].tsx` 一致、不改键盘机制。
- 别用「压矮内容区」换空间：布局属性逐帧重排会掉帧，详见 [[mobile-keyboard-and-viewport]] §11。

## 残留 / 待验证

- 300ms 是经验值 + 内容末尾 `pb-6` 兜底；真机若仍差几像素（或安卓 IME 动画更长），改 `KeyboardAwareScrollView` + `bottomOffset`。
- 2026-09-21 未实机走查（改动者本会话看不了截图），用户装包确认后才算闭环。
