---
title: sitin-pwa 聊天页路由判断 — /chat 无尾斜杠导致 gate 失配
date: 2026-08-17
tags: [troubleshooting, sitin-next, app-pwa, routing]
---

# sitin-pwa 聊天页 gate:`/chat` 无尾斜杠

## 坑
`app-pwa` 里想「聊天页不弹全局弹窗」,用 `pathname.includes("/chat/")`(带尾斜杠)判断,
结果弹窗照样在聊天页弹出。典型现象:`InsExchangeModal`「Social Requests」交换弹窗弹在
聊天页,与页内 `InsExchangeBubble` 气泡重复。

## 根因
sitin4.0 已把聊天统一成**单一路由** `CHAT_ROUTE = "/chat"`(`constants/routes.ts`),
**没有尾斜杠、也没有 `/chat/:id` 段**(旧的 `/chats` 列表 + `/chat-detail` 详情已合并)。
所以聊天页 `pathname === "/chat"`,`includes("/chat/")` 永远匹配不到 → gate 恒失效。

## 修法(判聊天页的正确姿势,全仓统一)
- `import { CHAT_ROUTE } from "@/constants/routes"`
- 用 `pathname.startsWith(CHAT_ROUTE)`(如 `GlobalAnomalyBanner`)
  或 `pathname === "/chat"`(如 `useOnlineStatusReport` / `useWorkStateMachine`)。
- **不要**用 `includes("/chat/")`。

## 命中位置(2026-08-17 修)
`packages/app-pwa/src/hooks/useInsTaskInit.ts` `canShowInsModal()`:
`pathname.includes("/chat/")` → `pathname.startsWith(CHAT_ROUTE)`。commit `0f068a251`。

## 排查提示
以后写「XX 页不做某事」的路由判断,先去 `constants/routes.ts` 确认真实 path 常量,
再和全仓已有判法对齐(grep `CHAT_ROUTE` / `"/chat"`),别凭印象拼字符串。
