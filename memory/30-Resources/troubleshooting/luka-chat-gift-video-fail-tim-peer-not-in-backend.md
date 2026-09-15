---
title: Luka 聊天页送礼/视频失败 = 会话对方不在 PWA 后端（TIM-only 测试号）
date: 2026-09-15
tags: troubleshooting, luka, chat, gift, video-call, tim, backend
---

# Luka 聊天页「送礼发不出去 / 视频打不通」= 会话对方不在 PWA 后端

- **现象**：聊天页点礼物 → 选一个 → Send → toast `Couldn't send the gift — try again`；
  点视频 → toast `Couldn't load video call options. Try again.`（视频面板根本不弹）。
  余额充足（1600 sparks）也会这样。
- **排查路径（可复用）**：
  1. 界面上先确认按钮接线没坏 —— 用代码里现成的深链 `luka://chat/<id>?bondTask=gift|video`
     直接开面板（`app/chat/[id].tsx` 的 `handledBondTaskRef`），能开 = UI 没问题。
  2. **日志落盘**，不用猜：`xcrun simctl get_app_container booted com.lukasoc.luka.dev data`
     → `Library/Caches/logs/session.log`，里面是 `[接口] req/resp <ProtoName> … code=…`
     的全量网络流水（`services/net-log.ts` → `@heyhru/rn-log`）。
  3. 关键三行（peer `22` / `2100047057` 这类号）：
     - `GetUserBasicInfosV2` → `userBasicInfos: []`（后端不认识这个人）
     - `UserDistance` → `code=99 "other user not found"`
     - `SendGift` → `code=100 "to user not found"`（`UserServiceCommonCode.USER_NOT_FOUND = 100`）
  4. 反证：在**后端认识的**用户会话里（Brittany `2100034740`）同样操作 →
     `SendGift code=1 leftCoin=…`，礼物气泡正常出现。
- **根因**：Chats 里那些 TIM 会话的对方是 **TIM-only 的测试号**（TIM SDKAppID 1600002475
  是全仓共用的测试 app），`api-dev.lukasoc.com` 的用户表里没有它们。客户端行为都正确：
  礼物在客户端余额检查通过后发出请求，被后端拒；视频在 `getPeerBasicInfo` 拿不到人 →
  没有 targetType → 直接 toast 不弹面板。
- **结论 / 下一步**：不是 app 的 bug。要么用后端认识的用户测（如 2100034740 / Aspen /
  Aliyah），要么让后端把 QA 的 TIM 测试号同步进 dev 用户表。
- **顺带发现**：`SendGift` 失败时客户端只给一句通用 toast（`services/gift.ts` 把非
  Success 都映射成 `error`，丢了 `code`/`message`）；`GetUserBasicInfosV2` 的空结果也
  没有提示。要更好定位，值得在 dev 构建里把后端 `message` 带进 toast。
