---
title: LangGraph JS 流式（streamMode messages / stream-events 协议）实测
date: 2026-10-14
tags: [langgraph, langchain, streaming, sse, koa-agent, troubleshooting]
---

# LangGraph JS 流式实测（langgraph 1.5.0 / core 1.2.17）

来源：`/Users/colna/TEST/agent-demo`（koa-agent）做 SSE 前的探针实测（2026-10-14）。

## 坑与结论

1. **classic `streamMode:"messages"` 只回聚合最终消息（无逐 token）**：LangGraph 经典路径挂的 `StreamMessagesHandler` 只设 `lc_prefer_streaming`；core 的 classic 流式分支会把 `_streamResponseChunks` 的 chunk 全部 `concat` 聚合后只调 `handleLLMEnd`（**不逐 chunk 调 `handleLLMNewToken`**）→ messages 模式每个 AI 消息只 yield 一条完整 `AIMessageChunk`。想拿逐 token 必须走 stream-events 协议。
2. **逐 token 正解（自定义 callback handler）**：自定义 handler 设 `lc_prefer_chat_model_stream_events = true` 并实现 `handleChatModelStreamEvent(event, runId)`；core 检测到后走 stream-events 分支，默认桥 `_streamChatModelEvents` → `convertChunksToEvents(_streamResponseChunks)` 产出 `content-block-delta`（`{ type:"text-delta", text }`）事件，逐 chunk 回调。LangGraph 的 `graph.stream(..., { callbacks: [handler] })` 会传播到节点，**节点必须把 config 传给 `model.invoke(messages, config)`**。
3. **`version:"v3"`**（`graph.stream(..., {version:'v3'})`）会启用 `StreamProtocolMessagesHandler`（`isV3 = options?.version === 'v3'` 判断于 pregel）；输出为协议信封（lifecycle `{id,step,source}` + 事件数组），解析复杂，非必要不选。
4. **core 里的 `baseMessages.length === 1` 是 batch 数不是消息数**：多轮（3 条消息）输入照常走流式分支，别被误导。
5. **`createReactAgent` 在 1.5.0 已 deprecated**（注释：moved to `langchain` 包）；`ToolNode` / `toolsCondition` 仍在 `@langchain/langgraph/prebuilt` 未废弃 → 手写 StateGraph 是无额外依赖的稳妥路线。
6. **`concat` 不再是 `@langchain/core/messages` 导出**：翻源码在 `@langchain/core/language_models/chat_models.js` 里 `import { concat } from "../utils/stream.js"` → 正确路径 `@langchain/core/utils/stream`。
7. **自定义 Fake 模型要覆写 `_streamResponseChunks` 且流式分片要带 `tool_call_chunks`**：流式分支聚合消息时只有 chunk 里出现过的东西才会进最终消息；工具轮如果只 yield 文本分片，聚合后 `tool_calls` 会丢，图就永远不进 tools 节点。

## 复现要点

```js
class TokenHandler extends BaseCallbackHandler {
  lc_prefer_chat_model_stream_events = true;
  handleChatModelStreamEvent(e) {
    if (e.event === 'content-block-delta' && e.delta?.type === 'text-delta') { /* e.delta.text */ }
  }
}
const stream = await graph.stream(input, { streamMode: 'updates', callbacks: [new TokenHandler()] });
```
