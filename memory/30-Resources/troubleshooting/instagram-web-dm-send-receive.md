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
