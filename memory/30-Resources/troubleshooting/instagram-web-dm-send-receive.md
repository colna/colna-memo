---
title: Instagram Web DM 发送/接收接口逆向
date: 2026-10-21
tags: troubleshooting, instagram, dm, graphql, mqtt, realtime, web
---

# Instagram Web DM 发送/接收接口逆向

## 发送（HTTP GraphQL）

- mutation：`IGDirectTextSendMutation`
- doc_id：`26911679871773184`
- endpoint：`POST https://www.instagram.com/api/graphql`
- variables：

```json
{
  "data": {
    "ig_thread_igid": "<内部 thread fbid>",
    "offline_threading_id": "<唯一数字串>",
    "recipient_igids": null,
    "replied_to_client_context": null,
    "replied_to_item_id": null,
    "reply_to_message_id": null,
    "sampled": null,
    "text": { "sensitive_string_value": "<消息文本>" },
    "mentions": [],
    "mentioned_user_ids": [],
    "commands": null,
    "forwarded_from_thread_id": null,
    "is_forwarded_from_own_message": null,
    "send_attribution": "igd_web_chat_tab:in_thread"
  }
}
```

- 响应：`data.xig_direct_text_send_with_slide_messaging_response { message_id, timestamp_ms, id }`
- `ig_thread_igid` 是内部 thread id，**不是** URL 里的 `thread_key`。
  - `IGDThreadDetailQuery`（doc `39768212509458705`）返回：
    - `thread_key` = URL `/direct/t/<thread_key>/`
    - `thread_fbid` = 发送用的 `ig_thread_igid`
- 发送走 HTTP，不走 MQTT；需要 sessionid + csrftoken + `x-ig-app-id: 936619743392459` + 页面 GraphQL 表单 token。

## 接收（页面内事件）

页面内有单例 EventEmitter：

```js
requireLazy(["getIGDNotificationEventEmitter"], (m) => {
  const emitter = m.default ? m.default() : m;
  emitter.on("newMessage", (ev) => {
    // ev: { threadFbid, messageId, senderFbid, senderName,
    //       senderProfilePicUrl, textBody, timestampMs, mediaPreviewSrc }
  });
});
```

- 由 `IGDSlideDeltaNewMessageHandler` 在新消息 delta 时触发；事件字段已经解析好，不需要自己解 MQTT/protobuf。
- 页面刷新/整页导航后监听器会丢，WebView 注入时需要在每次页面加载后重新注册。

## 底层实时通道（仅作了解）

- Direct 实时有两条：
  - `wss://gateway.instagram.com/ws/realtime`（DGW）：`streamcontroller` 订阅 `FBGQLS:XDT_DIRECT_REALTIME_EVENT`，doc `28367799366176340`，当前 routing hint 是 `IGDTypingIndicatorClientSubscription`（typing/presence）。
  - `wss://edge-chat.instagram.com/chat?sid=<session>&cid=<deviceId>`：MQTT 3.1.1 over WebSocket；CONNECT username 是 JSON（`u`/`a`/`d`/`s`/`cp`/`ct`/`fg` 等），订阅 `/t_ms`，通过 `/messenger_sync_create_queue`/`/messenger_sync_get_diffs` 做 sync。
  - 另有 `wss://gateway.instagram.com/ws/lightspeed` 做 mailbox sync（protobuf，`LSResponse`/`OnMailboxDelta`）。
- 自己复刻 MQTT/lightspeed 成本高；优先 hook `getIGDNotificationEventEmitter`。

## 实测

- 发送请求已用无效 thread 验证：服务端返回 GraphQL 业务错误（不是鉴权错误），说明 mutation/doc_id/headers 正确。
- `getIGDNotificationEventEmitter` 已用合成 `newMessage` 事件验证监听生效。

## 2026-11-12 更新：Slide 架构下的静默通知与聊天记录（已实测）

当前 IG Web 把 Direct 换成了 Slide 模型（`SlideMailbox` / `SlideThread` / `SlideMessage`），旧的 `IGD*` 命名仍在，但 doc_id 已轮换。以下为 2026-11-12 线上构建实测值。

### 新消息通知（页面内事件）

- `getIGDNotificationEventEmitter` 仍是单例 `EventEmitter`（模块源码：`var e=null; function s(){ e==null&&(e=new EventEmitter()); return e }`）。
- `IGDSlideDeltaNewMessageHandler` 收到新消息 delta 后执行：
  `getIGDNotificationEventEmitter().emit("newMessage", {mediaPreviewSrc, messageId, senderFbid, senderName, senderProfilePicUrl, textBody, threadFbid, timestampMs})`
- 出站消息也会 emit；按 `senderFbid !== viewer.id` 过滤自己（`viewer.id` 在 thread/inbox 响应里，等于 `ds_user_id`）。
- 桌面构建的模块 loader 是闭包，`window.requireLazy`/`window.require` 不可见；注入脚本能否拿到 emitter 取决于宿主构建，需运行时探测。拿不到时用 DOM 红点 MutationObserver（现有 `startDMListener` navbar/inbox 模式）兜底，零网络。

### GraphQL doc_id（2026-11-12，随 IG 构建轮换）

| 用途 | Operation | doc_id |
|---|---|---|
| 会话列表 + 每会话最近 5 条 | `PolarisDirectInboxQuery` | `28881837051457834` |
| 单会话最近 20 条 | `IGDThreadDetailQuery` | `29620840297508799` |
| 翻旧消息 | `IGDMessageListAnchorMessageRangeBeforeOffMsysQuery` | `30055859477336770` |
| 增量拉新 | `IGDMessageListAnchorMessageRangeAfterOffMsysQuery` | `29162554043360959` |
| 未读 badge | `IGDBadgeCountOffMsysQuery` | `28939180542359933` |
| 消息请求 | `PolarisDirectMessageRequestQuery` | `30071460809109024` |
| 会话 header | `IGDInboxHeaderOffMsysQuery` | `28914252884904227` |
| inbox tray | `IGDInboxTrayQuery` | `29231580869776032` |
| chat tab root | `IGDChatTabRootOffMsysQuery` | `29075227342105452` |

### 请求形状

`POST https://www.instagram.com/api/graphql`，`application/x-www-form-urlencoded`：

```text
av=<ds_user_id>            # 实测用 ds_user_id 即可（viewer.id）
__d=www&__user=0&__a=1&__req=<random>
fb_dtsg=<HTML 里 DTSGInitialData.token>
jazoest="2"+sum(charCode(fb_dtsg))
lsd=<HTML 里 LSD.token>
fb_api_caller_class=RelayModern
fb_api_req_friendly_name=<Operation>
server_timestamps=true
variables=<JSON>
doc_id=<id>
```

Headers：`x-csrftoken`（cookie `csrftoken`）、`x-fb-lsd`、`x-ig-app-id: 936619743392459`、`x-asbd-id: 359341`、`x-fb-friendly-name`。响应可能带 `for (;;);` 前缀。device id 取 `localStorage["chatd-deviceid"]`。

### 会话列表 variables

```json
{
  "device_id_for_iris_subscription": "<chatd-deviceid>",
  "__relay_internal__pv__IGDIsProfessionalAccountGKrelayprovider": false,
  "__relay_internal__pv__IGDPinnedThreadsRenderEnabledGKrelayprovider": false,
  "__relay_internal__pv__IGDMaxUnreadMessagesCountrelayprovider": 5,
  "__relay_internal__pv__IGDThreadListActionsEnabledGKrelayprovider": false
}
```

响应（非专业账号）`data.get_slide_mailbox_for_iris_subscription.threads_by_folder.edges[]`：

- `node.id` / `node.__typename=SlideThread`
- `node.as_ig_direct_thread`：
  - `thread_fbid`：发给 `IGDThreadDetailQuery` 的 id
  - `thread_key`：URL `/direct/t/<thread_key>/`，1:1 时等于对方内部 fbid（**不是** thread_fbid）
  - `thread_id`、`is_group`、`folder`、`system_folder`、`last_activity_timestamp_ms`、`marked_as_unread`、`is_muted`、`slide_read_receipts`
  - `users[]`：`id`（igid）、`username`、`full_name`、`profile_pic_url`
  - `viewer`：`id`（=ds_user_id）、`username`、`profile_pic_url`
  - `slide_messages.edges[]`：最近 5 条，`node.{message_id,sender_fbid,timestamp_ms,text_body,igd_snippet,content_type,content.__typename}`；图片只有 `igd_snippet`，没有媒体 URL
  - `page_info`
- `client__is_unread` / `client__unread_messages_count` 是 Relay client resolver，**原始响应不返回**；不要依赖。轮询检测新消息请比较每 thread 最新 `message_id`。

### 单会话消息 variables

```json
{
  "thread_fbid": "<thread_fbid>",
  "min_uq_seq_id": null,
  "__relay_internal__pv__IGDEnableOffMsysChatThemesQErelayprovider": false,
  "__relay_internal__pv__IGDInitialMessagePageCountrelayprovider": 20
}
```

响应 `data.get_slide_thread_nullable.as_ig_direct_thread`：`thread_fbid`、`thread_key`、`thread_id`、`viewer`、`users`、`marked_as_unread`、`slide_read_receipts`、`last_activity_timestamp_ms`、`slide_messages.edges[]`（最新在前，最多 20 条）、`page_info.{start_cursor,end_cursor,has_previous_page,has_next_page}`。

消息字段：`id`/`message_id`（`mid.$...`）、`sender_fbid`（内部 fbid；自己=viewer.id）、`timestamp_ms`、`text_body`、`content_type`、`content`、`reactions`、`replied_to_message`、`slide_edit_history` 等。

媒体：`content.__typename=SlideMessageImageContent` → `content.attachments[].{preview_cdn_url,attachment_cdn_url,preview_width,preview_height,attachment_fbid}`；视频/音频/贴纸/多图在 `content` 的对应 inline fragment 里。

### 翻旧 / 增量 variables

Before（doc `30055859477336770`）：

```json
{
  "id": "<thread_fbid>",
  "first": 20,
  "before": null,
  "after": null,
  "last": null,
  "older_than_message_id": "<已知最老一条 message_id>",
  "newer_than_message_id": null,
  "include_edge_message": false,
  "is_anchor_message": true
}
```

响应 `data.fetch__SlideThread.as_ig_direct_thread.slide_messages_before.edges[]`（不含 anchor，最新在前）。翻页把 `older_than_message_id` 设为上一页最后（最老）一条的 `message_id`；`before` 可置 null，实测同样有效。

After（doc `29162554043360959`）：同 shape，把 `newer_than_message_id` 设为已知最新一条 `message_id`，响应在 `slide_messages_after.edges[]`（不含 anchor）。增量同步重复直到 `page_info.has_next_page=false`。

### 实测证据（2026-11-12）

- 空账号：inbox query 200，`threads_by_folder.edges=[]`。
- 真实 1:1 thread（3 条消息：图片 + 两条文本）：
  - inbox query 返回 1 条 thread，`thread_fbid=2123654601589298`、`thread_key=17844610005665568`、`viewer.id=26460493204`，`slide_messages` 3 条。
  - thread detail 200，返回 3 条消息，`page_info` 两个 cursor，`marked_as_unread=false`，`slide_read_receipts=[]`，未返回 client resolver 字段。
  - Before 以最老 message_id 为 anchor → 0 条；以最新 message_id 为 anchor → 2 条（正确排除 anchor）。
  - After 以最老 message_id 为 anchor → 2 条（正确排除 anchor）。
  - `id` 传 `thread_fbid` 有效；无需 Relay base64 id。

### 风控

- 全部只读；不要调用 `useIGDMarkThreadAsReadMutation`、不要发消息。
- 分页/轮询加 1–2s 抖动；不要高频刷 inbox。
- doc_id 随构建轮换；上线做契约探测 + DOM fallback。

## 2026-11-12 补充：全量会话/全量消息与 EIMU 坑

### 全量会话列表（thread list 分页）

- 首屏：`PolarisDirectInboxQuery`（28881837051457834）返回前 15 个 thread + `get_slide_mailbox_for_iris_subscription.id`（SlideMailbox id，实测 = viewer 的 `interop_messaging_user_fbid` / EIMU）。
- 翻页：`IGDThreadListOffMsysPaginationQuery`，doc_id **29461858216751557**（非专业账号）。variables：
  `{ id: <mailbox.id/EIMU>, count: 15, cursor: <上一页 page_info.end_cursor>, folder: "INBOX", newer_than_timestamp_ms: null, __relay_internal__pv__IGDPinnedThreadsRenderEnabledGKrelayprovider:false, __relay_internal__pv__IGDMaxUnreadMessagesCountrelayprovider:5, __relay_internal__pv__IGDThreadListActionsEnabledGKrelayprovider:false }`
  响应：`data.fetch__SlideMailbox.threads_by_folder.edges[]` + `page_info`；`has_next_page` 为 true 时用 `end_cursor` 继续。
- 专业账号：`IGDThreadListProfessionalOffMsysPaginationQuery` 28833611742996989 / `IGDThreadListProfessionalOffMsysThreadsBySystemFolderPaginationQuery` 28892059663797409（未实测）。

### 全量消息（单 thread）

- `IGDThreadDetailQuery` 取最近 20 条 + `page_info.start_cursor`。
- Before：`IGDMessageListAnchorMessageRangeBeforeOffMsysQuery`，`id=thread_fbid`、`older_than_message_id=当前最老 mid`、`include_edge_message=false`；每次返回的 edges 最老一条成为下一次 anchor；edges 为空即到底。
- 长会话按页循环，建议 1–2s 抖动 + 页数/时间预算（脚本默认 50 页 / 240s）。

### 关键坑：outgoing sender_fbid 是 EIMU，不是 ds_user_id

- thread detail 的 `viewer.id` = `ds_user_id`（如 26460493204），但消息 `sender_fbid` 用的是 `viewer.interop_messaging_user_fbid`（EIMU，如 18109385050885205）。
- 判断 `sender: "me"/"them"` 必须用 `thread.viewer.interop_messaging_user_fbid`；用 `viewer_id` 会把「自己发的」误判成对方，服务端可能对自己的消息自动回复。
- 已修：`insGraphql.getContext()` 增加 `viewerEIMU`（`CurrentUser.getEIMU()`）；`getChatMessages` 用 `thread.viewer.interop_messaging_user_fbid`；`startDMListener` 通过 context → sessionStorage → inbox query（`mailbox.id`）解析 EIMU，再过滤 outgoing。
- 实时 `newMessage` 事件同样用 EIMU 过滤；`listener status` 现在返回 `viewerEimu`。

## 2026-11-12 补充：静默发消息（IGDirectTextSendMutation 当前构建）

- 当前 doc_id：**28737274835943013**（旧的 26911679871773184 已轮换）。
- **变量是扁平的，不是 `{ data: { ... } }`**。这是本次最大的坑：包 `data` 会返回 `missing_required_variable_value`。
- 实测可用的 variables：

```json
{
  "commands": null,
  "forwarded_from_thread_id": null,
  "ig_thread_igid": "<thread_fbid>",
  "is_forwarded_from_own_message": null,
  "mentioned_user_ids": null,
  "mentions": null,
  "offline_threading_id": "<Date.now()+6位随机>",
  "recipient_igids": null,
  "replied_to_item_id": null,
  "reply_to_message_id": null,
  "send_attribution": "igd_web_chat_tab:in_thread",
  "text": { "sensitive_string_value": "<文本>" }
}
```

- 响应：`data.xig_direct_text_send_with_slide_messaging_response: { message_id, timestamp_ms, id }`。
- `thread_fbid` 可用 URL 的 `thread_key` 先过 `IGDThreadDetailQuery` 解析；也可从 inbox query 按 username 找。
- 实测发送成功：`silent-send-test` → `mid.$cAH8z98-EC3WoAOKHtRjnSjEeEVbf`（thread 2123654601589298）。发送后 thread detail 中该消息 `sender_fbid = viewer.interop_messaging_user_fbid`（EIMU），不是 ds_user_id。
- 实现：`sendMessage` 在路由前先走 API；解析不到 thread/insGraphql 不可用才回退 DOM。API 成功后不导航、不做 DOM post-check，回复由 DM listener 实时捕获。
- console：`__IG_DM.send(threadFbid, text)`。
