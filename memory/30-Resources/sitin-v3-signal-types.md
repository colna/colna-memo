---
title: sitin social-proxy V3 SignalType → 动作 → Payload 速查
date: 2026-06-29
tags: [sitin-next, social-proxy, v3, signal, reference]
---

# sitin social-proxy V3 SignalType → 动作 → Payload 速查

V3 管线:**Signal(事实)→ Todo(决策)→ Behavior(执行)**。真源:
- 枚举:`app-social-proxy-server/src/v3/signal/signal.entity.ts`(`SignalType`)
- 映射表:`src/v3/todo/definitions/index.ts`(`SIGNAL_TO_DEFS` —— 哪个 Signal 产哪些 Todo)
- 各 Def 的 `fromSignal` 读 payload 字段
- Admin 下发入口:`POST /v3/admin/signal/create`(`src/v3/dashboard/admin.controller.ts`),前端 `/social-proxy/actions` 页

> **关键:13 个 SignalType 只有 5 个在 `SIGNAL_TO_DEFS` 里有映射、会真正触发下游 Todo/Behavior。其余 8 个是审计记录或系统内部自动产生,手动派发只写一条空 Signal,无动作。**

## 会触发动作的 5 个(手动下发有意义)

| SignalType | 含义 | 触发动作(优先级串行) |
|---|---|---|
| `CE_EXCHANGED` | CE 交换完成(完整获客链路) | 回关 FOLLOW_BACK(100) → 静音 MUTE_USER(95) → 开场私信 SEND_DM(90) |
| `IG_DM_RECEIVED` | 收到 IG 私信 | 有 userId:CHAT_GATE(100)先跑,REPLY_DM(80)等门控完成再回复;无 userId:CLOSE_CHAT 关会话 |
| `IG_NEW_MESSAGE` | 客户端上报有新消息 | PULL_NEW_MESSAGE(拉新消息) |
| `PROFILE_SYNC_NEEDED` | 需同步创作者主页帖子图 | FETCH_PROFILE_IMAGES(抓主页图) |
| `TODO_FAILED` | 某 Todo 执行失败 | TODO_FAILED(重试,优先级衰减,最多 3 代,系统内部用) |

## 不触发动作的 8 个(纯审计 / 系统自产)

`IG_FOLLOW_RECEIVED`(收到关注,审计) · `CREATOR_ONBOARDED`(入驻,无映射) · `KEEPALIVE_TICK`(心跳) · `REFLOW_TRIGGER`(重排,无映射) · `INBOX_CHECK_TRIGGER`(由 inbox-poll 定时自产) · `INBOX_CHECKED`(检查完成,审计) · `FOLLOW_BACK_COMPLETED`(回关完成,审计) · `IG_ACCOUNT_RESTRICTED`(账号被限,告警/审计)

## Payload 字段(只填 payload 内字段;creatorId/userId/deviceId 是顶层独立传)

### CE_EXCHANGED
| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `userIgHandle` | string | 强烈建议 | 男用户 IG handle;缺省回退空串,回关/私信拿不到对象 |
| `isFirstContact` | boolean | SendDm 必需 | **必须 `true`** 才真正发开场私信,否则 SendDm 跳过 |

```json
{ "userIgHandle": "john_doe", "isFirstContact": true }
```
顶层需带 `userId`,否则 mute/私信关联不到用户。

### IG_DM_RECEIVED
`userIgHandle` · `messageText` · `messageType`(默认 text) · `igMessageId` · `timestamp`(ISO,默认 now) · `mediaUrl`(可选) · `signalError`(有值→走 CloseChat 不回复) · `messages[]{from,messageText,messageType,igMessageId,timestamp}` · `chatHistory[]{userId,content,timestamp,mid?}` · `windowSize`

```json
{ "userIgHandle": "john_doe", "messageText": "hey", "messageType": "text", "igMessageId": "ig_123", "timestamp": "2026-06-29T01:00:00Z" }
```
顶层带 `userId` → 门控+回复;不带 → 关闭会话。

### IG_NEW_MESSAGE
payload 为空,Def 只用顶层 creatorId。
```json
{}
```

### PROFILE_SYNC_NEEDED
| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `creatorIgHandle` | string | ✅ | 缺省 Def 返回 null,不产生动作 |
| `maxPosts` | number | 否 | 抓取上限,默认 DEFAULT_MAX_POSTS |

```json
{ "creatorIgHandle": "creator_jane", "maxPosts": 12 }
```

### TODO_FAILED(系统内部,不该人工构造)
```json
{ "failedTodoId": "...", "failedTodoType": "SEND_DM", "originalPriority": 90, "originalSignalId": "...", "retryGeneration": 1, "originalPayload": {} }
```

## 实务

Admin 手动下发最常用:`CE_EXCHANGED`(`userIgHandle`+`isFirstContact:true`+顶层 `userId`)、`PROFILE_SYNC_NEEDED`(`creatorIgHandle`)、`IG_NEW_MESSAGE`(空 payload)。审计类 8 个一般不人工派发。

> 接口经 minerva 反代是**双层 success**,前端要判内层。详见 [2026-06-26 工作日志](../50-Daily/2026-06-26.md)。
