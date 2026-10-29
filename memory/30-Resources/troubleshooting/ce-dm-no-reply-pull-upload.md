---
title: CE 后男方私信没自动回复：PULL_NEW_MESSAGE 成功但没有 CHAT_HISTORY
date: 2026-10-29
tags: [troubleshooting, sitin-next, social-proxy, ce, instagram, dm-listener, reply-dm]
---

# CE 后男方私信没自动回复

## 现象

- 环境：**dev**（`dev-presence-service-log / dora-dev-stdout`）
- 男：`2100071313`（IG `lilybrown4783`）
- 女：`2100069412`（Eva，`leosmith0909`）
- CE 订单 `60363` 已完成；回关成功；男方发 `Hi` 后女方没有自动回复

## 结论

这次**不是 handle 问题**：`FOLLOW_BACK` 成功（`alreadyFollowing=true`）。DM listener 也正常上报了 `NEW_MESSAGE`，`PULL_NEW_MESSAGE` 成功拉到 32 条消息，但 **native `uploadMessages` 没有回传 `CHAT_HISTORY` SCRIPT_EVENT**，所以服务端没有创建 `IG_DM_RECEIVED` / `REPLY_DM`，自动回复断在设备侧上传这一步。

另外 `MUTE_USER`（clickMute）连续超时失败，触发 `[Todo:cancel-sleeping] cancelled=1`，把同 signal 的 `SEND_DM`（女方主动打招呼）取消了，所以女方也没有先发第一条。

## 证据链（dev，UTC / 北京时间）

| UTC | 北京 | 日志 |
|---|---|---|
| 02:44:11 | 10:44:11 | CE `finishOrder` orderId=60363，`publishCeExchangedEvent` |
| 02:44:12 | 10:44:12 | `[Signal] type=CE_EXCHANGED`，`[Identity] → handle=lilybrown4783` |
| 02:44:33 | 10:44:33 | `[APK:result] followUserById success=true ... alreadyFollowing=true` |
| 02:45:33~02:49:45 | 10:45~10:49 | `MUTE_USER` clickMute 连续 4 次 60s 超时 → `[Todo:fail]` → `cancel-sleeping cancelled=1` |
| 02:46:04 | 10:46:04 | `SCRIPT_EVENT NEW_MESSAGE source=navbar badgeCount=1` → `IG_NEW_MESSAGE` → `PULL_NEW_MESSAGE` |
| 02:47:27 | 10:47:27 | `SCRIPT_EVENT NEW_MESSAGE source=inbox threads=[{lilybrown4783, "Hi", unread}]` → 第二次 `PULL_NEW_MESSAGE` |
| 02:48:34 | 10:48:34 | `getChatMessages success=true newMessages=32`，最后一条 incoming 是 `Hi` |
| 之后 | — | **没有任何 `CHAT_HISTORY` / `IG_DM_RECEIVED` / `REPLY_DM`** |

对比 prod 同一时段（creator=6171344）：
```
PULL_NEW_MESSAGE done
SCRIPT_EVENT request_id=upload_... data.type=CHAT_HISTORY
[Route] CHAT_HISTORY → IG_CHAT_HISTORY
[Signal] type=IG_DM_RECEIVED → REPLY_DM
```
→ prod 有 native `upload_...` 事件，dev 这对没有。

## 为什么会这样

1. `getChatMessages` 脚本只负责拉消息；`AndroidBridge.uploadMessages(...)` 落本地 DB 后，由 `ActionDispatcher.startUploadIns` 用 `sendNativeEvent("upload_...")` 发 `CHAT_HISTORY`。
2. dev 这次 `uploadMessages` 没有产生事件，可能是：
   - 设备 APK 版本较旧，native `uploadMessages`/outbox 未实现或未生效；
   - 本地 DB 里这些消息已经是 `uploaded=1`（`getAllUnuploaded` 为空），所以不重发；
   - outbox/WS 发送失败（设备日志会有 `uploadMessages saved` / `sent batch` / error）。
3. 服务端 `DeviceGateway.handleScriptResult` 也没有兜底直接路由 `getChatMessages` 的结果：`shouldRoute` 只看 `result.data.interaction`、`result.data.data.type`，而该结果是 `data.data.interaction=true` 且没有 `type`，即使走到 `routeInteractionToV3` 也会因 `Missing type` 跳过。

## 修复建议

### 设备侧（先查）

- 升级 dev 测试机 APK 到包含 `BridgeSyncActions.uploadMessages` / `ActionDispatcher.startUploadIns` 的版本。
- 在机器人 WebView 里确认 `typeof AndroidBridge.uploadMessages === "function"`。
- 看设备日志是否有：
  - `uploadMessages saved: inserted=... duplicates=...`
  - `uploadMessages: sent batch of N ... requestId=upload_...`
  - 或 `uploadMessages: no unuploaded messages`（说明本地 DB 认为已上传，需要清 DB 或补发）
- 也可以先清掉该 chat 的本地上传标记后重试一次。

### 服务端（建议加固）

- `handleScriptResult` 的 `shouldRoute` 增加 `result.data?.data?.interaction === true`。
- `parseInteractionPayload` 对 `getChatMessages` 结果补充 `type: "CHAT_HISTORY"`，并把 `messages` 归一化到 `data`（或让脚本直接返回 `type` + `data`）。
- `PULL_NEW_MESSAGE` 的结果如果 native 没有上传，服务端可以直接走一次 `IG_CHAT_HISTORY` 路由，不依赖设备 outbox。

### 解耦 SEND_DM

- `MUTE_USER` / 其他兄弟 Todo 失败时，不要连带取消 `SEND_DM`；至少把 `SEND_DM` 改成独立重试或延迟执行。否则回关成功也可能没有第一条 DM。

## 排查命令

```bash
# 这对的完整链路
/usr/bin/python3 ~/.agents/skills/ce-exchange-diagnose/scripts/trace_pair.py \
  --env dev --female 2100069412 --male 2100071313 --handle lilybrown4783 --minutes 4320

# 只看拉取/上传/回复
/usr/bin/python3 ~/.agents/skills/ce-exchange-diagnose/scripts/slsq.py --env dev \
  --service social-proxy-server \
  --query '"creator=2100069412" and ("PULL_NEW_MESSAGE" or "CHAT_HISTORY" or "IG_DM_RECEIVED" or "REPLY_DM")' \
  --from '2026-10-29 02:40:00' --to '2026-10-29 03:10:00' --reverse
```

## 关联

- `packages/app-social-proxy-server/src/gateway/device.gateway.ts`（`handleScriptResult` / `shouldRoute` / `routeInteractionToV3`）
- `packages/app-social-proxy-server/src/v3/signal/adapters/websocket.adapter.ts`
- `packages/app-ins-scripts/src/instagram/methods/getChatMessages.js`
- `GraceChat-Earn-Android/haven/src/main/java/com/harbor/prod/socialproxy/bridge/ActionDispatcher.kt`
- `GraceChat-Earn-Android/haven/src/main/java/com/harbor/prod/socialproxy/bridge/BridgeSyncActions.kt`
- 前一篇：[[ce-follow-back-invalid-ig-handle]]
