---
title: RN 输入框在滚动区、按钮在滚动区外：键盘起时按钮盖住输入框
date: 2026-09-21
tags: [troubleshooting, react-native, keyboard, sitin-rn, luka]
---

# RN 输入框在滚动区、按钮在滚动区外：键盘起时按钮盖住输入框

来源：sitin-rn `apps/luka/src/app/settings/delete.tsx`（删号确认屏），2026-09-21 用户录屏报「按钮压在输入框上」。同一失败类此前在 koda 举报页出现过（memo 2026-09-03 R00272，PR #375）。

## 现象

键盘弹起、焦点在确认框（滚动区**末尾**），屏底部钉着的操作栏（`Delete my account` / `Keep my nest`）正好压住输入框下半截，输入框下面那行提醒完全看不到。

用户第二轮反馈：「往上滚动的时候成两段滚动了，应该按钮和内容一起往上滚动」——即第一版修法（KAV 压矮 + focus 后延时 scrollToEnd）虽然把输入框救出来了，但按钮先跳、内容后滚的观感不对。终稿改成整块 `KeyboardStickyView` 平移，见下「选型」。

## 第一版的根因（三层，缺一不可）

1. `KeyboardAvoidingView(behavior="padding")` 只把滚动区**视口压矮**，不会让聚焦输入框滚进可视区。iOS 的 UIKit 自动滚只把输入框滚到「键盘上沿」，**不知道固定栏的高度**，于是输入框落在栏底下。
2. 栏的底部 padding 用 `useBottomInset()`（非键盘感知）：键盘起时仍留 `max(insets.bottom, 16)`（刘海机 34pt），把栏额外抬高 26pt，等于再从滚动区抢走 26pt。键盘态该用的是 `useKeyboardAwareBottomInset()` → `KEYBOARD_GAP = 8`。
3. focus 时没有任何「把输入框滚出来」的动作，滚动位置就停在 UIKit 默认滚到的位置。

## 终稿（luka 删号屏）

`settings/delete.tsx`：

```tsx
<View className="flex-1 overflow-hidden">
  <KeyboardStickyView style={{ flex: 1 }} offset={{ opened: restingInset - KEYBOARD_GAP }}>
    <ScrollView ref={scrollRef} contentContainerClassName="px-5 pb-6" ...>
      <DeleteLetterHero ... />
      <DeleteConfirmField onFocus={() => scrollRef.current?.scrollToEnd({ animated: true })} ... />
    </ScrollView>
    <View className="px-5" style={{ paddingBottom: restingInset }}>
      <DeleteActionBar ... />
    </View>
  </KeyboardStickyView>
</View>
```

`delete-confirm-field.tsx`：`TextInput` 透传一个可选 `onFocus`。

## 用哪种键盘避让：看用户要「整体上移」还是「内容自己滚」

这三条在 sitin-rn 里都真实用过，且被用户来回否决过，别重推：

| 结构 | 观感 | 什么时候对 |
|---|---|---|
| `KeyboardAvoidingView(behavior="padding")` + 普通 ScrollView | 底栏先贴到键盘上沿，滚动区随后被压矮 | **用户会读成「两段」**：按钮跳一下、内容再滚一段。且普通 ScrollView 不会把聚焦输入框滚进视野（koda R00272 v2 因此被否） |
| `KeyboardAwareScrollView` + `bottomOffset=固定栏高` | 框内滚动，把聚焦输入框滚到栏上方 | 要「聚焦输入自动避开键盘」且接受框内滚动（koda R00272 终稿） |
| **`<View flex-1 overflow-hidden>` + `KeyboardStickyView style={{flex:1}}` 包「滚动区 + 固定栏」** | **按钮与内容作为一整块随键盘平移**，动画只碰 `transform`，零逐帧重排 | **用户说「按钮和内容一起往上滚」时**（luka 聊天页、luka 删号屏终稿） |

要点：

1. 外面那层静止的 `overflow-hidden` 窗口是必须的 —— 整块上移后顶部溢出，没有它就会盖住 header。
2. `offset.opened = restingInset - KEYBOARD_GAP`：底栏的 resting 安全区 padding 被键盘盖住，把它从位移里扣掉，只留 `KEYBOARD_GAP`(8) 的缝，别让底栏浮在键盘上方 26pt。
3. 底栏的 padding 用**非**键盘感知的 `useBottomInset()`：键盘态 inset 若变化，底栏高度会在升起中途变，`onLayout` → 重渲 → 内容在动画最后一帧重排（`chat-input-bar.tsx` 的注释原话）。高度恒定，位移交给 `offset`。
4. 整块平移时**滚动区高度不变**，所以 `scrollToEnd` 的目标位置与键盘无关 —— `onFocus` 里可以直接滚（不需要等键盘动画的 `setTimeout`）；内容不超一屏时它是空操作。

## 残留 / 待验证

- 2026-09-21 未实机走查（改动者本会话看不了截图），用户装包确认后才算闭环。
- 内容超一屏（小屏 / 大字号）时靠 `onFocus` 的 `scrollToEnd` 兜底；若仍差几像素，改 `KeyboardAwareScrollView` + `bottomOffset = 栏高`。
