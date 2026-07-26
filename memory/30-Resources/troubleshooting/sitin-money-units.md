---
title: sitin app-pwa 金额字段单位一览(及 100 倍 bug 排查思路)
date: 2026-07-23
tags: [troubleshooting, sitin-next, app-pwa, money, cents, formatCents]
---

# sitin app-pwa 金额字段单位一览

**教训**:后端 proto 里叫 `*Cents` 的字段,单位**不都是美分**;写代码时先查这张表再决定要不要 `toRewardCents`。

## 单位速查

| 字段 | 单位 | UI 展示前 | 埋点(美元)|
|------|------|-----------|-------------|
| `Reward.cents` | 万分之一美元 | `toRewardCents(v)` → 美分 → `formatCents` | `toRewardDollars(v)` |
| `endChat.deductCents`(预估) | 万分之一美元 | 同上 | 同上 |
| `EndChatResponse.deductedCents`(实扣) | 万分之一美元 | 同上 | 同上 |
| `ConversationState.totalEarnedCents` | ⚠️ 存疑 | `EndChatModals` 是 `/100`,其他地方要单独查 | — |
| 余额 API `cash` | 美元(字符串) | `dollarsToCents(Number(cash))` | 直接用 |

## 通用换算函数(位置)

- `packages/app-pwa/src/hooks/useChatConv.ts:66` — `toRewardDollars(raw) = toRewardCents(raw) / 100`
- `packages/app-pwa/src/hooks/useChatConv.ts:57` — `toRewardCents(raw) = (raw ?? 0) / 100`
- `packages/app-pwa/src/hooks/useChatConv.ts:71` — `centsToDollars(cents) = cents / 100`
- `packages/app-pwa/src/utils/money.ts` — `formatCents(cents)` / `splitCents` / `formatEarnedCents` / `dollarsToCents`
  - **入参一律是美分**(≥100 显示 `$X.XX`,<100 显示 `X¢`)
  - 别把万分之一美元当美分喂进来,会放大 100 倍

## 已知历史 bug

- **2026-07-08**:5 处金额格式化实现互相矛盾,收敛到 `money.ts`,`ChatBody` 的 `+¢0.50` 修成 `+50¢`(PR ?)
- **2026-07-23**:`EndChatModals` 的 `deductCents` / `deductedCents` 展示成 `$9.90`(实际 9.9¢),PR #702 修 → 走 `toRewardCents`

## 排查 100 倍 bug 的经验

1. **文档不是真相** —— sitin4 技术设计文档一度把 `deductedCents` 注释为「美分」,和实际后端行为不符。看文档更要看**运行时 vConsole 数值 vs UI 展示**。
2. **`grep "formatCents" | grep -viE "toRewardCents|/ ?100"`** 一把梭,能命中所有"直接把服务端 cents 喂进去"的可疑点。
3. **同一弹窗多个金额同源不同单位** —— 相邻两行可能一个走 `/100` 一个没走,历史迁移遗留。修的时候要确认每个字段单独的单位口径,别一口气全 `toRewardCents`。

## 相关

- 知识库既有:[[sitin4-endchat-backend-gap]](拉黑接口契约)、[[pwa-female-wallet-billing]](余额口径)
- 本次改动 PR:https://github.com/presence-io/sitin-next/pull/702
