---
title: RefreshControl 的 spinner 会永久卡住 —— 下拉处理函数提前 return 时没发过 refreshing=true
date: 2026-09-23
tags: [react-native, refreshcontrol, luka, native-state]
---

# RefreshControl 的 spinner 会永久卡住 —— 下拉处理函数提前 return 时没发过 refreshing=true

**症状**（Luka T0121，2026-09-22）：非会员在通知页 Activity / Visitors 段**下拉**，手势把系统 spinner 拉了出来，随后弹出订阅页；从订阅页返回后，spinner 一直转在列表顶上，怎么滚都不消失。

**根因**：iOS `RCTRefreshControl.m` 的 `setRefreshing:` 只在 `_currentRefreshingState != refreshing` 时才走 `beginRefreshing` / `endRefreshing`。非会员的下拉处理只 `openLikedPaywall(...)` 就 `return`，**JS 从未把 `refreshing` 置为 true** → JS 侧 0 次状态变化 → 原生收不到任何 `endRefreshing`，手势自己拉起的那个 spinner 就永久留在原地。Android 侧同理：状态机只认 JS 的 `refreshing` 翻转。

**修法（两层，缺一不可）**：

1. **显式接管**：给被锁定的下拉加一个本地状态（`paywallPull: "activity" | "visitors" | null`），进付费墙前 `setPaywallPull("segment")`（effect 里立刻收），列表 `refreshing={list.refreshing || paywallPull === "segment"}`。这样至少保证 JS 发过一次 `true→false`。
2. **回屏重挂兜底**：`pullEpoch`（回屏时 +1）做 RefreshControl 的 `key`，让旧控件整个卸载重挂 —— 原生控件的状态会随视图销毁一起清掉，不依赖「那次 `endRefreshing` 命令有没有送达」。被订阅页盖住 / 手势被翻页打断的窗口里，命令可能丢；这层是保险。

**判据**：凡是下拉回调里存在**提前 `return`**（未订阅、无权限、空数据……）的列表，都要检查 `refreshing` 是否发过 `true` —— 没发过就必然卡 spinner。

**出处**：`apps/luka/src/app/notifications.tsx`，提交 `addd24c70`（第一层）+ `71665bf07`（第二层）；口径记在 `apps/luka/docs/notifications.md`《列表手势》。
