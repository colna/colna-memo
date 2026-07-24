---
title: CE INS 交换 — TIM 消息、接口与数据类型全表
date: 2026-07-24
tags: [reference, sitin-next, app-pwa, contact-exchange-service, ce, instagram, tim, 接口]
---

# CE INS 交换 — TIM 消息、接口与数据类型全表

> **口径**:代码事实为准,逐行核实过。
> - 前端:`sitin-next/packages/app-pwa`,分支 `feature/sitin4.0`,commit `eadc65d44`(2026-07-24)
> - 后端:`presence-io/contact-exchange-service`,commit `8467dbd`(Kotlin + Spring Boot + R2DBC + gRPC:9096)
> - Proto:`sitin-next/packages/business-pwa-proto/proto/archat_api/{user_api,contact_exchange_api}.proto`
>
> 相关:[[ce-ins-exchange-flow]](流程图 + 与需求文档的冲突清单)

---

## 0. 一句话总览

**CE 交换全程由 PWA 自驱**:TIM 自定义消息只做「invalidate 信号」,权威数据永远来自 `listUserInsExchangeOrder`;钱由 `finishAndFollowInsExchangeOrder` 发;真实 follow 由 APK 内 WebView 小崽执行。CE 后端只在「DH 请求 / 退款」两类系统场景主动发 TIM。

---

## 1. TIM 消息全表

传输形式统一为 `TIMCustomElem`:`payload.description` = CustomDescription,`payload.data` = JSON 字符串。

### 1.1 CustomDescription 是什么

TIM 原生只有 `TIMTextElem` / `TIMImageElem` / `TIMSoundElem` 等几种元素,**没有「交换卡片」这类业务消息**。所有业务卡片都塞进万能的 `TIMCustomElem`,靠 `payload.description` 这个字符串标签区分是哪种业务:

```
TIM 消息
├─ type: "TIMCustomElem"                       ← SDK 层:这是一条自定义消息
└─ payload
   ├─ description: "freeExchangeRequest"       ← 业务层类型标签(即 CustomDescription)
   └─ data: "{\"orderId\":123,\"accountID\":\"xxx\"}"   ← 业务数据,JSON 字符串
```

`CustomDescription` 枚举(`app-pwa/src/types/chatMessage.ts:41-73`)把这些魔法字符串收敛成常量,收发两端共用。整个 PWA 里这类标签 30 多个(`Gift`、`CallOrder`、`VideoTips`…),交换相关占 6 个。

**一个标签决定三件事**:
1. **用哪个类解析** —— `formatMessage()` 工厂(`:687-748`)按 `description` switch
2. **渲染成什么气泡** —— class 的 `type` 决定走哪个 bubble;`belongsToChatMsg = false` 的不进聊天列表
3. **离线推送文案** —— `IMManager.sendCustomMessage`(`:596`)查 `descriptionMessageTypeMap` 自动生成

### 1.2 6 个标签 = 3 种角色 × 2 代命名

`chatMessage.ts:67-72`

| CustomDescription | 解析类 | 实际发送方 | 说明 |
|---|---|---|---|
| `insExchangeRequest` | `ExchangeRequestMessage` | 男端 App / dora;CE 反欺诈退款 + DH 请求也复用 | 老版付费交换请求 |
| `freeExchangeRequest` | `ExchangeRequestMessage` | **PWA 自己发**(`useInsTaskInit.ts:329`) | 新版(Blurred Card V2)交换请求 |
| `insExchangeSend` | `ExchangeSendMessage` | 老版男端,新版 PWA 已不发 | 接受方回执 |
| `freeExchangeSend` | `ExchangeSendMessage` | **PWA 自己发**(`useInsTaskInit.ts:517`) | 接受方回执,给出自己的 IG 号 |
| `insExchangeSystem` | `ExchangeSystemMessage` | CE 后端 `notifyExpiredRefund` | 过期退款通知 |
| `freeExchangeSystem` | `ExchangeSystemMessage` | **PWA 自己发**(`InsExchangeBubble.tsx:77`) | 拒绝通知 |

**3 种角色**(真正的区别):

| 角色 | 谁发 | 干什么 |
|---|---|---|
| `*Request` | 发起方 | 「我想跟你换 IG」邀请卡,收方看到 Accept / Reject + 倒计时 |
| `*Send` | 接受方 | 「我同意了,这是我的 IG 号」回执卡,收方看到 Follow 按钮 |
| `*System` | 系统 / 拒绝方 | 纯通知(拒绝、过期退款),无按钮、`belongsToChatMsg=false` 不进列表 |

**2 代命名**(历史包袱):`ins*` = 第一代**男方付金币**买交换;`free*` = 第二代 Blurred Card V2**女方免费主动**发起。但 PWA 代码里两代 **`case` 落到同一个类、渲染完全一致**,真正区分付费/免费的是订单状态和 `orderStatus` 字段。**实质只有 3 种消息,每种两个历史别名。**

> ⚠️ **标签写错不报错**:`formatMessage` 的 `default` 返回 `UnknownMessage`,不抛错不告警 → 表现为「消息发出去了、SDK 也收到了、界面上什么都没有」。新增自定义消息必须同步改三处(枚举常量 / `formatMessage` case / bubble 组件),漏一处就静默失效。详见 [[troubleshooting/sitin-next-pwa-chat-tim]]。

### 1.3 交换请求 `ExchangeRequestPayload`

`chatMessage.ts:416-429`

| 字段 | 类型 | 说明 |
|---|---|---|
| `orderId` | number | 订单 ID,后续所有接口的主键 |
| `orderStatus` | string | `"unpaid"` / `"paid"`;女端看自己发出的卡片时用它决定头像是否打码 |
| `expireTimestamp` | number | 过期时间(ms),老字段 |
| `expireAt` | number | 过期时间(ms),新字段;取值 `expireAt ?? expireTimestamp` |
| `pwafollowReward` | number | 女方接受可得金额。**注意是小写 `f`**,不是 `pwaFollowReward` |
| `insAccount` | string | 发起方 IG 号,老字段 |
| `accountID` | string | 发起方 IG 号,新字段;取值 `accountID \|\| insAccount` |
| `insAvatar` | string | 发起方 IG 头像,老字段 |
| `avatarUrl` | string | 发起方 IG 头像,新字段;取值 `avatarUrl \|\| insAvatar` |
| `planType` | string | 实验分组标识,接受方回执时原样带回 |
| `cardType` | number | `1=IG` / `2=Snapchat` / `3=X` |
| `giftId` | string | **仅 CE 服务端 DH 场景**带,固定 `"rose"` |
| `localPwaStatus` | enum | **PWA 本地写回**的处理态:`countdown / agreed / refused / expired` |

### 1.4 接受回执 `ExchangeSendPayload`

`chatMessage.ts:469-480`

| 字段 | 类型 | 说明 |
|---|---|---|
| `orderId` | number | 对应订单 |
| `accountID` / `insAccount` | string | **接受方(女方)**的 IG 号 |
| `avatarUrl` / `insAvatar` | string | 接受方 IG 头像 |
| `planType` | string | 从原请求消息里读出后原样带回 |
| `cardType` | number | 同上 |
| `requestMsgId` | string | 回指原始请求消息的 TIM msgId |
| `pwafollowReward` | number | 奖励金额 |
| `localPwaStatus` | enum | `unfollowed / followed` |

### 1.5 系统消息 `ExchangeSystemPayload`

`chatMessage.ts:512-518`

| 字段 | 类型 | 说明 |
|---|---|---|
| `score` | number | 亲密度变更,拒绝时前端写死 `-1000` |
| `content` | string | 展示文案 |
| `orderId` | number | 对应订单 |
| `requestMsgId` | string | 回指原请求消息 |
| `reason` | enum | `expired` / `decline` |

`belongsToChatMsg = false`,不进聊天列表渲染。

### 1.6 `localPwaStatus` 的写回机制(易踩坑)

`useInsTaskInit.ts:112-149` 的 `updateMessageStatus()`:

```ts
const nextPayload = { ...chatMessage.payloadData, localPwaStatus: status };
// 直接修改 payload.data 属性,不替换整个 payload 对象
// SDK 内部通过原始对象引用追踪变更,替换对象会导致变更不被检测
rawMessage.payload.data = JSON.stringify(nextPayload);
await IMManager.modifyMessage(rawMessage);
```

作用:**跨端同步 + 防重复处理**。状态取值:请求侧 `countdown → agreed | refused | expired`;回执侧 `unfollowed → followed`。

### 1.7 CE 服务端主动发的 3 条 TIM

`contact-exchange-api/.../push/TimPushService.kt` + `push/CeNotificationService.kt`

走 TIM REST `POST /v4/openim/sendmsg`,admin 账号 `administrator_unread_enabled`,`SyncOtherMachine=2`(不同步发送方其他端)、`OnlineOnlyFlag=0`(离线也存储)。

| 场景 | Desc | content 字段 | 离线推送 |
|---|---|---|---|
| 反欺诈退款 `notifyFraudRefund` | `insExchangeRequest` | `orderId` / `coins` / `content` | 无 |
| 过期退款 `notifyExpiredRefund` | `insExchangeSystem` | `orderId` / `coins` / `content` | `Exchange failed😢.Received %s coins refund.` |
| DH 交换请求 `notifyDhExchangeRequest` | `insExchangeRequest` | `giftId:"rose"` / `insAvatar` / `insAccount` / `orderId` / `expireTimestamp` / `pwafollowReward` / `orderStatus:"unpaid"` | 无 |

> ⚠️ **男方真人下单那条 `insExchangeRequest` 不在 CE 仓库**。CE 只覆盖 DH 与退款两类系统场景,正常男方付费下单的卡片由男端 App / dora 侧发出。

---

## 2. 接口全表

PWA 侧统一走 HTTP + protobuf(`http/httpClient.ts` → `requestPost2`),按 proto ID 路由。封装在 `app-pwa/src/http/insApi.ts`。

### 2.1 订单主链路

| 函数 | proto ID | 请求 | 响应 | PWA 用途 |
|---|---|---|---|---|
| `listUserInsExchangeOrder` | 4652/4653 | `{}` | `orders: InsExchangeOrderFullInfo[]` | **权威数据源**,待处理订单列表 |
| `finishAndFollowInsExchangeOrder` | 4650/4651 | `{orderId, maleUserId}` | `{earnedAmount, maxEarnedAmount}` | **完成交换 + 发钱**(不可逆) |
| `rejectInsExchangeOrder` | 4658/4659 | `{orderId}` | `{giftCoinValue}` | 拒绝 → 男方金币退款 |
| `getPwaFollowedUser` | 4674/4675 | `{}` | `followedUserinfo[]` | 已关注列表(**仅 APK <1.34 老路径**) |
| `getClientConfig` | — | `{}` | `clientConfig` JSON 串 | 取 `ins_exchange_gift` 礼物配置 |

### 2.2 Blurred Card V2(女主动,`contact_exchange_api.proto`)

| 函数 | proto ID | 请求 | 响应 |
|---|---|---|---|
| `checkBlurredCardCondition` | 21016/21017 | `{maleUserId}` | `{code, message, eligible}` |
| `sendBlurredCardGift` | 21018/21019 | `{maleUserId, giftId}` | `{code, message, success}` |
| `createBlurredCardOrder` | 21020/21021 | `{maleUserId}` | `{code, message, orderId, expireTimestamp, pwaFollowReward}` |

> ⚠️ 这三个接口在 `contact-exchange-service` 仓库 **grep 零命中**,实现仍在 dora / user-service。改「女主动」逻辑别找错仓库。

### 2.3 PWA 侧死代码(封装了但零调用方)

| 函数 | proto ID | 说明 |
|---|---|---|
| `getInsExchangeConditionInfo` | 4660/4661 | 已被 `checkBlurredCardCondition` 取代 |
| `initInsExchangeOrder` | 4662/4663 | 素人两步下单的老路径 |
| `getPwaInsExchangeOrderReward` | 4676/4677 | 收益明细 |

`safePost` 包装(`insApi.ts:34-45`)失败时静默返回 `null` → **网络错误与业务失败无法区分**。

### 2.4 CE 后端 gRPC 全量 RPC(`ContactExchangeGrpcImpl.kt`,678 行)

响应统一 `code`(0 成功 / 1 失败 / 2 专属业务错误)+ `message`。

**条件检查(3)**
- `checkInsExchangeCondition` — 通用状态机
- `getInsExchangeConditionInfo` — PWA 详情版
- `getInsExchangeGiftInfo` — 礼物配置(被动接收方给 `firstOrder`,否则 `default`)

**订单管理(6)**
- `initInsExchangeOrder` — 素人建 INIT 单(100 年过期占位)
- `payInsExchangeOrder` — 男方支付 INIT 单(INIT→FINISH)
- `createAndPayInsExchangeOrder` — 男方一步创建+支付(→PAID,24h)
- `rejectInsExchangeOrder` — 素人拒绝(→REJECT)+ 同步退款
- `finishAndFollowInsExchangeOrder` — 完成 + 发 follow reward + 发 Kafka
- `pwaInsFollowEarn` — 单独补发 follow 收益
- `listUserInsExchangeOrder` — 待处理列表(按 maleUserId 去重保留最新)

**收益统计(3)**:`getUserInsTotalEarn` / `getPwaFollowedUser` / `getPwaInsExchangeOrderReward`

**INS 管理(3)**:`saveUserInsId`(带头像时下载→上传 GCS)/ `saveInsChatHistory` / `getInsChatHistory`

**消息收益(3)**:`pwaInsTextEarn` / `queryPreIncrBalance` / `confirmPreIncrBalance`

**跨模块检查(3)**:`checkInsExchange` / `checkNotInsPwaOrDhExchange` / `isMeetup`

> `guardEmpty()` helper 把 `Mono.empty()` 转成 INTERNAL error —— 否则 `subscribe(onNext, onError)` 两个回调都不触发,`responseObserver` 永不 `onCompleted()`,gRPC 客户端会无限挂起。

---

## 3. 六个问题的直接答案

### 3.1 男端发来的 TIM 有什么字段

见 §1.3。要点:新老字段并存(`accountID`/`insAccount`、`avatarUrl`/`insAvatar`、`expireAt`/`expireTimestamp`),`pwafollowReward` 小写 f,`localPwaStatus` 是 PWA 自己写回去的不是男端给的。

### 3.2 自动处理 → 上报后端接口 & TIM

`useInsTaskInit.ts:179-201`,收到请求消息**不直接用消息里的数据**,只当刷新信号:

```
收到 TIM ExchangeRequestMessage
  → bpTrack(pwa_instagram_request_message_passive)
  → checkPendingOrders()
     → listUserInsExchangeOrder()                    权威列表
     → dispatchInsExchangeOrders(msgs)               分流
        ├ createUserId == 我   → handlePeerAccepted()
        └ createUserId == 男方 → handleAcceptExchange()
             └ finishAndFollowInsExchangeOrder({orderId, maleUserId})
                → { earnedAmount, maxEarnedAmount }   💰 钱在这发,不可逆
                → Promise.all:
                   ① updateMessageStatus(msgId, "agreed")
                   ② IMManager.sendCustomMessage(freeExchangeSend)
                   ③ startRobot()  → APK SocialProxyWebView 真实 follow
```

拒绝路径(`InsExchangeBubble.tsx:63-90`):
`rejectInsExchangeOrder({orderId})` → `updateMessageStatus("refused")` → 发 `freeExchangeSystem`(`{score:-1000, content:"Exchange failed😢.", orderId, requestMsgId, reason:"decline"}`)。

### 3.3 「定期请求接口 女主动」

**代码里没有任何 setInterval 做这件事。** 触发器是聊天轮次(`useInsTaskInit.ts:165-172`):对方发一条 → 我回一条 = 一轮完成,立刻调 `initiateInsExchange(uid)`。

完整链路(`:285-362`):

| # | 动作 | 接口 | proto ID |
|---|---|---|---|
| 0 | 目标必须是真人 `UserType.User` | `getPeerUserInfo` | — |
| 1 | 校验是否允许触发 | `checkBlurredCardCondition {maleUserId}` → `{eligible}` | 21016/21017 |
| 2 | **创建交换订单** | `createBlurredCardOrder {maleUserId}` → `{orderId, expireTimestamp, pwaFollowReward}` | 21020/21021 |
| 3 | **PWA 主动发 TIM** `freeExchangeRequest`,payload = `{orderId, accountID, avatarUrl, expireAt, cardType:1}` | — | — |
| 4 | 持久化 `exchangedUsers` 到 UserCloudStorage | — | — |
| 5 | 埋点 `pwa_instagram_request_message_initiative` | — | — |
| 6 | 连发礼物(`getClientConfig` 取 `ins_exchange_gift.count`) | `getClientConfig` | — |
| 7 | 亲密度加成 | `sendBlurredCardGift {maleUserId}` | 21018/21019 |

**三问三答**:能触发交换 ✅ / 能创建交换订单 ✅ / 会发 TIM ✅。

「看起来像定期请求」的原因:`exchangedUsers` 是去重集合,入队前先 `add`,但 `eligible === false` 或抛异常时会 `delete` → **下一轮聊天再问一次**;只有成功发出交换请求才落盘不再重复。步骤 4 之后的 5/6/7 属非关键路径,失败不影响交换。

### 3.4 男端接收也会发 TIM

女方接受后 PWA 发 `freeExchangeSend`(字段见 §1.4)。对端收到 `ExchangeSendMessage` 且 `isInsLoggedIn === true` 时(`:197-201`)走 `handlePeerAccepted()`(`:543-580`):

```
finishAndFollowInsExchangeOrder(orderId, peerUserId)
  → Promise.all:
     ① startRobot({orderId, peerUserId, insAccount:"", insAvatar:""})
     ② updateMessageStatus(msgId, "followed")
```

> ⚠️ 若此时 `isInsLoggedIn === false`,**该消息被直接丢弃且无补偿**,只能靠 `listUserInsExchangeOrder` 的 `isFollowOrder` 分支救回。

另外 CE 服务端会发 §1.7 的 3 条系统 TIM。

### 3.5 PWA 批量接受接口

**不存在批量接口。** `components/InsExchangeModal.tsx` 的 "Accept all to earn":

```
handleAcceptAll()
  → onAccept([...pendingMessages])
     → handleAcceptExchange(messages, skipRewardModal)
        → for (const msg of messages)                    ← 前端 for 循环
             await insExchangeQueue.add(async () => {     ← 串行队列
               finishAndFollowInsExchangeOrder(msg.orderId, msg.peerUserId)
               ...
             })
        → 累加 earnings → 统一弹一次 showRewardModalAsync
```

单个 Accept 按钮走同一条路径,只是数组长度为 1。`dispatchInsExchangeOrders` 只负责把 follow 单 / accept 单拆成两组 `Promise.allSettled`,组内仍是串行。**要真批量必须后端新开 RPC。**

### 3.6 checkCondition 判断是否支持交换

有两个同名不同物的 RPC,别混:

| RPC | proto ID | 返回 | PWA 是否在用 |
|---|---|---|---|
| `checkInsExchangeCondition` | 4648/4649 | `status`(LOCK/UNLOCK/PROCESSING/COMPLETED)+ `orderId` + `createUserId` + `isMeet` | ❌ 未封装 |
| `getInsExchangeConditionInfo` | 4660/4661 | `allow` + `condition{isInsPwa, isVideoTest, leftChatRound, leftTriggerCount}` + `isMeet` | ❌ **封装了但零调用方** |

**`checkInsExchangeCondition` 判定顺序**(`InsExchangeService.kt:47-113`):

1. 扫两人间历史订单 —— 有 `FINISH` → `COMPLETED`;有未过期的 `INIT`/`PAID` → `PROCESSING`
2. 查 `userinfo` 的 `ins_id` + `cai_user_type`,`isInsPwa = ins_id 非空 && cai_user_type == 1`
3. 真 INS PWA:`chemistry.getCoinConsume >= insPwaMinCoin(500)` → `UNLOCK`,否则 `LOCK`
4. 非 INS PWA:`chemistry.getChatRound >= notInsPwaChatRound(200)` → `UNLOCK`,否则 `LOCK`

**`getInsExchangeConditionInfo` 额外逻辑**(`:117-200`):
- 女方无 `ins_id` → 直接 `allow=false`
- Byteplus A/B `ins_test_switch` 关闭 → `allow=false`
- Meetup 捷径:`chat_meetup` 已 `DATING_CONFIRMED` 且无进行中订单 → `allow=true`(需 `meetupStrategyEnabled` + `pwaMeetupStrategyEnabled` 双开)
- 否则查聊天轮次,`< pwaSendChatRound(5)` → `allow=false` 且回 `leftChatRound`
- 最后查是否已有被动单/进行中单 → `allow = !exist`

**现网女方侧实际判据是 `checkBlurredCardCondition`**,上面两个都不走。

另有三个内部检查给别的服务调:`checkInsExchange`(通用)/ `checkNotInsPwaOrDhExchange`(DH 场景,阈值 20 轮)/ `isMeetup`。

---

## 4. 数据类型

### 4.1 订单状态 `InsExchangeOrderStatus`

| 值 | 含义 |
|---|---|
| `INIT` | 素人占位单,100 年过期,只等男方支付 |
| `PAID` | 男方已付金币,待素人 follow,24h 过期 |
| `FINISH` | 已完成;`followed` 字段区分是否已点「已关注」 |
| `REJECT` | 素人主动拒绝,已退金币 |
| `REFUNDED` | 过期自动退款 |

### 4.2 条件状态 `InsConditionStatus`

`LOCK(0)` / `UNLOCK(1)` / `PROCESSING(2)` / `COMPLETED(3)` / `COOLDOWN(4)`
> `COOLDOWN` 只在 proto 里有,CE 的 `toProtoConditionStatus` 未映射。

### 4.3 `InsExchangeOrderInfo`(列表接口返回)

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | int64 | 订单 ID |
| `create_at` / `update_at` / `expire_at` | int64 | 时间戳 |
| `create_user_id` | int32 | 创建者(**双向可创建**,PWA 靠它区分 follow 单 / accept 单) |
| `male_user_id` / `female_user_id` | int32 | 双方 |
| `status` | string | 见 4.1 |
| `followed` | bool | 是否已 follow |
| `relate_gift_id` / `relate_gift_coin` | string / int32 | 关联礼物 |
| `time_message_id` | string | 关联的 TIM 消息 ID |

外层 `InsExchangeOrderFullInfo` = `male_user_info`(UserInfo)+ `order` + `pwa_follow_reward`。

### 4.4 其他枚举

- `CardType`:`CARD_TYPE_IG(1)` / `CARD_TYPE_SNAPCHAT(2)` / `CARD_TYPE_X(3)`
- `PwaUserBalanceChangeType`:`INS_FOLLOW_REWARD` / `INS_MESSAGE_TEXT`
- `PwaInteractionType`:`MUTED` / `BLOCKED`(RoaringBitmap base64 存 Redis,TTL 90 天)
- `SparkStatus`:`EXTINGUISHED / LIT / FADING / REIGNITING`
- `CoinChangeType`:`USER_INS_EXCHANGE_COST` / `USER_INS_EXCHANGE_ORDER_REFUND`

### 4.5 Kafka 事件 `ce.exchanged`

`finishOrder` 事务提交后 fire-and-forget 发出,social-proxy 消费触发 FOLLOW_BACK:

```json
{
  "creatorId": "女方 userId",
  "userId": "男方 userId",
  "platform": "ig",
  "userIgHandle": "男方 IG handle",
  "creatorIgHandle": "女方 IG handle",
  "exchangedAt": "ISO 8601"
}
```

### 4.6 阈值与默认值 `CeProperties`

| 配置项 | 默认 | 用途 |
|---|---|---|
| `threshold.notInsPwaChatRound` | 200 | 非 INS PWA 解锁轮次 |
| `threshold.insPwaMinCoin` | 500 | 真 INS PWA 解锁金币 |
| `threshold.pwaSendChatRound` | 5 | PWA 可触发轮次 |
| `threshold.notInsPwaOrDhChatRound` | 20 | DH 场景轮次 |
| `order.defaultExpireSeconds` | 86400 | PAID 单 24h 过期 |
| `gift.firstOrder` | `id_watch` / 150 | 被动接收方礼物 |
| `gift.default` | `roses` / 450 | 默认礼物 |
| `insOfficial.defaultInsId` | `iranoble49` | 官方号,用于判「真实 INS PWA」 |
| `schedule.expiredOrderCron` | `0 * * * * *` | 每分钟扫过期单 |

---

## 5. 完整流程

```
【入口 A】男方 App 内付费下单(PWA 不可见)
  └→ CE createAndPayInsExchangeOrder
       事务: lockUserCoin → existOrderProcessing → INSERT(PAID, +24h) → deductCoin
       事务后: logCoinChange / delCoinConsume / resetChatRound / handleMeetupCooldownBlacklist
  └→ TIM insExchangeRequest ────────────────┐
                                            │
【入口 B】女方 PWA 聊天满一轮(对方发→我回)  │
  └→ getPeerUserInfo(必须 UserType.User)    │
  └→ checkBlurredCardCondition → eligible    │
  └→ createBlurredCardOrder → orderId        │
  └→ TIM freeExchangeRequest ───────────────┤
  └→ 礼物 x N + sendBlurredCardGift          │
                                            │
                          女端收到,只当信号 ▼
                     checkPendingOrders()
                       └→ listUserInsExchangeOrder()   ← 权威
                       └→ isInsLoggedIn ?
                            false → 存 modal / 等待小崽启动
                            true  → dispatchInsExchangeOrders()
                                     ├ isFollowOrder → handlePeerAccepted()
                                     └ 其他          → handleAcceptExchange()
                                            │
        ┌───────────────────────────────────┴───────────────────────────────┐
     Accept                                                              Reject
  finishAndFollowInsExchangeOrder                                rejectInsExchangeOrder
   ├ checkFraud(chat_meetup 已确认约会 + 真 INS PWA → 强制退款+BLOCKED)      ├ PAID → REJECT
   ├ 事务: lockBalance → finishAndFollow → addBalance → recordBalanceChange  └ 同步 refundOrder
   ├ 事务后: removeBlackUser(MUTED) / 发 Kafka ce.exchanged                  └ TIM freeExchangeSystem
   └ 返回 earnedAmount / maxEarnedAmount        💰 不可逆
        │
        ├→ TIM freeExchangeSend(带女方 IG 号)
        ├→ updateMessageStatus("agreed")
        └→ startRobot() → APK SocialProxyWebView → instagram.com + app-ins-scripts
                                                    真实 follow + 发招呼
                                            │
                            男端收到 freeExchangeSend
                              └→ handlePeerAccepted → finishAndFollow + startRobot
                              └→ updateMessageStatus("followed")

【超时】CE Scheduler 每分钟 findExpiredOrders → refundOrder → REFUNDED
        PAID 单退款后发 TIM insExchangeSystem + 离线推送
```

---

## 6. 已知问题

1. **钱货两清风险**(`useInsTaskInit.ts:515-523`)—— `finishAndFollowInsExchangeOrder`(钱已发、不可逆)与 `startRobot`(悬浮窗权限最终未授予会静默 `return`)在同一个 `Promise.all` 里。订单已 FINISH 不再进待处理列表,`checkPendingOrders` 救不回来,**无补偿无重试**。
2. **无执行结果回调** —— `startSocialProxyRobotWebView` 只返回「是否启动成功」,机器人 follow 失败 PWA 完全不知道。
3. **谁去 follow 仍有重叠** —— CE 发 `ce.exchanged` 让 social-proxy 回关,PWA 又自己 `startRobot()` 关一遍,文档没交代边界。详见 [[ce-ins-exchange-flow]] §4.3。
4. **消息永久丢弃**(`:197-201`)—— `ExchangeSendMessage` 到达时若 `isInsLoggedIn === false` 直接丢。
5. **被封 == 未登录**(`:371`)—— 冷启动探针只判 `type === 0`,把「被封/禁言」和「未登录」合并处理,都引导重新登录。
6. **`safePost` 静默吞错** —— 网络错误与业务失败无法区分,全返回 `null`。

---

## 7. 关键文件索引

### 前端 `sitin-next/packages/app-pwa/src/`

| 文件 | 角色 |
|---|---|
| `hooks/useInsTaskInit.ts` | **CE 总调度**(715 行),由 `useUserInit.ts` 在 TIM 登录后启动 |
| `hooks/useInsBridgeCallbacks.ts` | APK → PWA 授权回调 |
| `http/insApi.ts` | 全部 CE HTTP 接口封装 |
| `types/chatMessage.ts` | `:414-530` Exchange 三类消息定义 |
| `components/InsExchangeModal.tsx` | 批量 Accept 弹窗(前端循环) |
| `pages/Chat/bubbles/InsExchangeBubble.tsx` | 聊天内 Accept / Reject 卡片 + 倒计时 |
| `utils/insExchangeQueue.ts` | 串行队列 |
| `utils/bridge.ts` | `:166-289` INS native 桥 |
| `stores/insStore.ts` | `insState`(已授权)/ `isInsLoggedIn`(会话可用),**两者正交** |

### 后端 `contact-exchange-service/`

| 文件 | 角色 |
|---|---|
| `api/grpc/ContactExchangeGrpcImpl.kt` | gRPC 入口,678 行,20+ RPC |
| `api/service/InsExchangeOrderService.kt` | 订单生命周期 554 行(创建/支付/拒单/完成/退款/欺诈) |
| `api/service/InsExchangeService.kt` | 条件检查 + INS ID + 聊天记录 |
| `api/service/InsExchangeRewardService.kt` | 收益计算 + 预加钱 |
| `api/push/TimPushService.kt` | TIM REST 推送 + UserSig 生成(HMAC-SHA256 + Deflate + Base64URL) |
| `api/push/CeNotificationService.kt` | 3 个业务通知场景 |
| `api/configuration/InsExchangeScheduler.kt` | **唯一定时任务**:每分钟扫过期单退款 |
| `api/mq/CeEventProducer.kt` | Kafka `ce.exchanged` |
| `infra/repository/InsExchangeOrderRepository.kt` | 订单表 CRUD + `FOR UPDATE` 行锁 |
| `docs/` | 11 篇设计文档(architecture / grpc-api / order-lifecycle / data-model 等) |
