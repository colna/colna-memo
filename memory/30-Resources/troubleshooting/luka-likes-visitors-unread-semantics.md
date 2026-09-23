---
title: Luka likes/visitors 未读语义 —— 服务端 per-row isUnread 会清，徽标「原数据+新增」只会出在客户端清零时机
date: 2026-09-23
tags: [troubleshooting, luka, sitin-rn, notifications, badge, backend-data, protobuf]
---

# 症状

飞书 T0135（P1，09-22）：【likes&visitors】只有一条数据时，新增 1 条后展示的红点数 = 原数据 + 新增数据。
用户确认口径：**进段看过旧的一条之后，新来一条，外部徽标仍算 2**（应为 1 / 0）。
相关：T0124（红点应在当前页面就消失）、T0048（徽标与名单不一致）。

# 结论（三层）

1. **服务端没问题**：`MarkMatchLikedRead` 是 bodyless 的 mark-ALL，调用后 `GetMatchLikedList`
   的 `totalUnreadLikedUserCount → 0` 且**每一行的 `isUnread → false`**；再来新 like 时
   `total=2 / totalUnread=1`，旧行 false、新行 true。不存在「旧的不清」。
2. **likes 徽标口径**（B1，`71a075ddd`）：数 `GetMatchLikedList` 第一页里 `isUnread` 且未被
   `filterOutMatched`（已在聊）隐掉的行；不再读 `totalUnreadLikedUserCount`（含被隐掉的人）。
   visitors 徽标数 `GetVisitorList` 里 `lastVisitTime > visitorTime` 的行；不再读
   `GetNewVisitorCount`（那是**访问事件数**，同一个人看两次算 2，名单只有 1 行 —— T0048 的
   「徽标 1、列表 2 个红点」就是它）。**⚠️ 2026-09-23 修正：`lastVisitTime` 后端下发的是
   毫秒，客户端基线是秒 —— 该比较修复前恒为真，见文末《T0150 根因》。**
3. **客户端清零时机是唯一会漏的地方**：
   - 进段即清（`9e86478ea`，用户 2026-09-22 口径，推翻 09-21 的「离开才清」）；
   - **屏内兜底每段各一条**：人还在该段上、名单里又出现未读行就继续清（Activity →
     `markAllRead() + clearLike()`；Visitors → `markVisitorsSeen()` 拨 baseline）。
     **T0135 就是 Visitors 缺这条兜底**：停留期间每次新访问都往徽标上加，旧的一条不清 →
     「原数据 + 新增数据」。已补（`apps/luka/src/app/notifications.tsx` 里
     `tab === "visitors"` 的 effect）。
   - 旧包（≤09-22 17:44 的构建）是「离开才清」，页内停留时旧数一直可见，新来一条同样变 2 ——
     测试同学的包在 T0124 视频里可见 5 颗红点页内不消失，所以旧包上 Activity 也会复现。
     换含 `9e86478ea`+B1 的包即可验证。

# 复现/取证手法（可复用，无需 App/模拟器）

`/tmp/t0135-probe/probe.ts`（esbuild 打包后 node 跑）：

1. esbuild alias 指向共享包源码：
   `--alias:@heyhru/common-util-pb=./packages/common-util-pb/src/index.ts --alias:@heyhru/business-pwa-proto=./packages/business-pwa-proto/src/index.ts`
2. `PbClient` + `FastLoginRequest { deviceId: 随机串, platform: iOS } { skipAuth: true }` mint 新 dev 账号；
   **uid 在 `res.userInfo.userId`**（不是 `res.userId`），token 在 `res.token`。
3. **造 like 的最短路径**：账号 B 对 A 调
   `SwipeCardRequest { userId: A.uid, swipeType: SwipeType.SwipeTypeRight, viewDuration: 3000 }`
   —— profile 没填完也能点（我们两个新账号 gender=0、未 complete，照样成功）。
   一个账号一个人只能点一次；再造一条就 `FastLogin` 第三个设备号。
4. 用 A 的 token 依次拉 `GetMatchLikedList` / 调 `MarkMatchLikedRead` / 再拉，打印
   `totalUnreadLikedUserCount` 与每行 `isUnread`，即可证明服务端语义。
5. 访客**可以造**（2026-09-23 实测通过，脚本 `/tmp/t0150-probe/visitor-probe6.ts`）：账号 B 调
   `ReportVisitorRequest { visitorInfos: [{ visitorId: B.uid, visitedId: A.uid, actionType:
   VISITOR_PROFILE }] }`（**不要带 `delayTime`**；带 `delayTime: 120000` 的没落库），A 再拉
   `GetVisitorList` 就有行。`GetUserProfilePageInfo` 本身不记访客（返回 code=0）。

# 2026-09-23 续：T0150 根因 —— `lastVisitTime` 是毫秒，客户端基线是秒

- **症状**：测试同学「visitors 红点无法消失，一直存在」；Visitors 分段 tab 无徽标、但
  **每张访客卡都有红点**；返回 Chats 稍等后爪印重新出现（截图 22 = 全量访客；底栏 63 =
  消息 41 + 22）。
- **实测**（dev 探针，同上第 5 条造访客）：`lastVisitTime = 1790151879016`，同时刻本地
  `now = 1790151879` 秒 —— **后端下发 epoch 毫秒**；而 `Visitor.lastVisitTime` 的类型契约、
  `visitor-time.ts` 基线与 `GetNewVisitorCount.startTime`（proto 注释「unix 时间戳 秒」）都是
  **秒**。`毫秒 > 秒` 恒真 ⇒ 一切访客永远「未读」：卡片常红；进段清零把 store 擦成 0（tab 无
  徽标）但卡片清不掉；离屏 `refresh()` 再算出全量 ⇒ 爪印「一直存在」。这就是 T0048 / T0124 /
  T0135 反复打回的共同底因（此前只按「清零时机」和「徽标口径」修，从未验过字段单位）。
- **修法**：共享引擎 `packages/business-vibes/src/vibes.ts` 的 `getScouted()` 归一
  `visitTimeSeconds()`（`>= 1e12` 视为毫秒，否则原样透传 —— 对自己的后端下发秒的 app 无害）；
  mapper 一处修，Luka 与所有消费方一起好。测试 +2（毫秒换算 / 秒透传）。
- **同步文档**：`packages/business-vibes/README.md`《访客时间是秒》、
  `apps/luka/docs/notifications.md`《徽标》。
- **验证要点（真机）**：进 Visitors 后卡片红点/爪印应在下次 refresh 后归 0；停留在 Visitors
  时新访客按 T0135 兜底当场清；离开后新访客仍计 1（口径不变）。

# 相关

- 飞书：T0135（record `recvvX5X7cBQBQ`）、T0124（`recvvWjjT3OXmu`）、T0048（`recvvPE7rfIeRO`）。
- `apps/luka/docs/notifications.md`《徽标》《分段徽标与默认段》。
- T0048 的 B1 决策与「服务端汇总 vs 名单」两条错口径的由来见 `10-Projects/luka-rn-app.md`。
