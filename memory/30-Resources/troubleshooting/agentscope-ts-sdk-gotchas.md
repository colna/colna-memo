---
title: AgentScope TypeScript SDK 0.0.15 移植要点（Python 2.0.8 → TS）
date: 2026-09-23
tags: [agentscope, typescript, koa, sdk, migration, troubleshooting]
---

# AgentScope TypeScript SDK 0.0.15 移植要点

来源：把 Python 版 AgentScope 学习项目（`agentscope-learn`）移植为 TS 版（`agentscope-learn-ts`，官方 SDK `@agentscope-ai/agentscope@0.0.15` + Koa 3 + `node:sqlite` + Vitest 5）。

## 坑与结论

1. **`structuredModel` 只在类型里，Agent 没接线**（`ReplyOptions.structuredModel` 声明了但 `agent.ts` 未使用）。要用结构化输出必须走 `ChatModelBase.callStructured({ messages, schema })`（内部用工具调用强制 schema，返回 `{ type:'structured', content }`）。注意 `stream:true` 时它自己会消费流，调用方拿不到 delta。
2. **`Tool.requireUserConfirm` 默认 `false`**（Python `FunctionTool` 默认需要确认，语义相反）。演示确认流必须显式 `requireUserConfirm: true`；SDK 内置工具（Bash/Read/…）都是 true。
3. **最终 `Msg` 没有 `finished_reason`**：`reply()`/`replyStream()` 正常返回即完成；要判断结束原因只能看 `REPLY_END` 事件。`replyStream` 的最终 `Msg` 是生成器 `done` 时的返回值（Python `yield_final_msg=True` 的等价物）。
4. **`OpenAIChatModel` 默认 `maxRetries: 3`**：离线/故障注入模型要显式 `maxRetries: 0`，否则一个失败会重试 4 次（测试计数全错）。
5. **`LocalFileStorage` 是目录布局**：`<saveDir>/<agentId>/context.jsonl + state.json`；`agent.saveState()` 在 `replyStream` 的 `finally` 自动调用。做「恢复后重放确认」场景时要给只读 storage（`saveAgentState` 空实现），否则 finally 会把工具结果写回覆盖断点。
6. **状态快照恢复 replyId**：Agent 公开 `context`/`replyId`/`curIter`/`curSummary`；自定义 `StorageBase` 单文件存 `{context, metadata}`，恢复时注入新 Agent，`loadState()` 会带回 replyId，确认事件用这个 reply_id 提交。
7. **工具调用异常会被 SDK 转成错误工具结果回传模型**（与 Python 一致），不会从 `runWithConfirmation` 冒泡；想断言异常要直接调用工具函数或在 ask 层注入错误。
8. **Koa 3 没有 FastAPI 自动校验/文档**：请求体用 zod `.strict().trim()` 手动校验并返回 422 `{detail}`；SSE 用 `ctx.respond=false` + `res.writeHead` 原生写。
9. **Node 内置 `node:sqlite`**：`DatabaseSync` 同步 API 与 Python sqlite3 接近，但无 `backup()`；测试快照用 `copyFileSync`（journal_mode=delete 下单文件复制安全）。Node 22 有 ExperimentalWarning，脚本里用 `NODE_OPTIONS=--disable-warning=ExperimentalWarning` 静音。
10. **课程独立移植测试技巧**：`OfflineModel extends OpenAIChatModel` 覆写 `_callAPI`（public），支持异步 responder 做 gated 时序控制；`startServer(app)` 随机端口 + 全局 `fetch` 跑真实 HTTP。

## 复现命令

```bash
cd /Users/colna/WORK/agentscope-learn-ts
pnpm typecheck && pnpm lint && pnpm test   # 28 files / 161 tests
```
