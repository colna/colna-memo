---
title: update_bot 改 instructions 报 blocked for runs someone else started
tags: [rakazo, bot-config, troubleshooting]
created: 2026-11-02
---

# update_bot 改 instructions 被平台拦截（run 归属）

## 坑

给 bot 写 `instructions`（`update_bot`）时返回：

```json
{"error":"Instruction changes are blocked for runs someone else started."}
```

同一会话里连续 3 次都是同一句；换内容、换字段、改长度均无效。

## 根因

拦截条件看的是 **run 的发起者**，不是请求内容：只有**用户本人直接发起**的 run 才允许改 bot 配置。由另一个 bot（如「调度」）转发/派发产生的 run 属于 "someone else started"，一律拒绝。

识别线索：回复被标注 `[to 调度]`、或上游消息来自 teammate bot —— 该 run 归他人发起。

## 修法

1. 不要在他人发起的 run 里反复重试（同一策略错误，重试零收益）。
2. 定稿存成文件（例 `tmp/memo-instr-btight.txt`），逐字可复用。
3. 回到**用户自己发起的对话**里执行一次 `update_bot`：用户在 bot 聊天窗口说「把 `tmp/xxx.txt` 写入你的 instructions，写完读回校验」。
4. 写后 `update_bot` 无参读回，做长度 + `startswith(原 instructions)` 校验，确认原文未丢、追加段完整。

## 证据（2026-11-02）

- 167 字原文 → 1731 字定稿：调度转发 run 中 3 次全部返回上述 error，写前写后读回均为 167 字。
- 同一定稿在用户直接发起的 run 中一次 `{"ok":true}`；读回 1731 字，`draft_equals_readback=True`、`prefix_167_preserved=True`。
