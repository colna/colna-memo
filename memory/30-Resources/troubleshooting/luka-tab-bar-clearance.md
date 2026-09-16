---
title: Luka 屏底留白：标签栏是 JS 浮层，别用裸 insets.bottom 让位
date: 2026-09-16
tags: [luka, react-native, layout, safe-area, tab-bar]
---

# Luka 屏底留白：标签栏是 JS 浮层，别用裸 `insets.bottom` 让位

**症状**：Chats 白卡列表划到底，最后几行被底部标签栏盖住（2026-09-16 用户实机截图：卡片被栏沿整齐切掉，末行只露半个头像）。

**根因**：`apps/luka/src/app/(tabs)/index.tsx` 的白卡下边距写的是 `insets.bottom + 8`。

- Luka 的标签栏在**所有平台**都是 JS 自绘的 `TabBar`（`components/tabs/tabs-navigator.ios.tsx` → `PillTabs`，iOS 从未接 NativeTabs），绝对定位浮在场景**之上**，高 `PILL_HEIGHT(63) + insets.bottom`。
- `useSafeAreaInsets().bottom` 只是 Home Indicator（iPhone 34pt），**不含**那 63pt。于是卡片底部的 55pt 被栏盖住。
- 触发注释里那句「iOS 26 上 `bottom` 已经含了系统标签栏（见 `clearance.ios.ts`）」是模板（blueprint）遗留注释 —— 对 Luka 不成立；`clearance.ios.ts` 与 `(tabs)/_layout.tsx` 里同款说法都别信。

**修法 / 约定**：tab 屏底部让位一律用 `@/components/tabs/clearance` 的钩子：

| 场景 | 用哪个 | 值（iPhone） |
| --- | --- | --- |
| 列表 / 卡片（普通滚动区） | `useTabBarClearance()` | `bottom + 63 + 12` |
| 包了一层 overlay View 的列表 | `useOverlayListClearance()` | 同上 |
| 浮层按钮贴住标签栏 | `useFabClearance()` | `bottom + 63 + 24` |

**禁止** `insets.bottom + 常量` 直接当底部让位 —— 那是「只给 Home Indicator 让位」，栏一浮上来就盖住内容。全仓 grep 兜底：`rg "insets.bottom \+" apps/luka/src`（当前仅 `waves-you-sent.tsx` 有，且那是被 stack 覆盖的推入路由，标签栏不可见，属正常）。
