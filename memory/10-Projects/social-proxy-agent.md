---
title: social-proxy-agent
date: 2026-10-14
tags: project, agent, langgraph, koa, sse, social-proxy
---

# social-proxy-agent

Social Proxy 内部运维 Agent：用自然语言查 SLS/SP 日志与报表、发信号、画图，带 Web 聊天界面。

## 基本信息

- 仓库：<https://github.com/colna/social-proxy-agent>（public；10-15 由 `koa-agent` 更名，包名/UI/localStorage key 同步）
- 路径：`/Users/colna/TEST/social-proxy-agent`
- 技术栈：Koa 3 + zod + LangGraph JS（DeepSeek OpenAI 兼容端点）+ pnpm workspace；web 为 React + ECharts
- 文档：`docs/00-代码导读.md`（架构、请求流、扩展练习）、`docs/01-04`（需求/技术/任务拆解/进度）
- 环境变量：`apps/server/.env`（`OPENAI_*`、`SP_ADMIN_*`、`SLS_*`、`KB_MEMO_DIR`、`SP_ADMIN_*_PROD`）

## 能力（截至 2026-10-16）

- **SP 工具**：脚本错误日志/报表/仪表盘/信号等，dev/prod 双环境（`env` 参数；prod 用 `SP_ADMIN_*_PROD`，未配置时返回指引）。
- **SLS**：`sls-logs` skill 工具化，OAuth 模式 + STS 自动刷新，dev/prod 双环境。
- **BytePlus**：查询工具接入（沿用 `byteplus-query` skill）。
- **图表**：`render_chart` 只回传 `ChartSpec`（line/bar/pie），前端 ECharts 渲染 + 历史回读还原。
- **RAG**：`kb_search` / `kb_read` 直接调 colna-memo 的 `./colna search` 混合检索，不在服务端重建 embedding。
- **交互**：多会话、重新生成、导出、附件（文本/图片；前端压缩长边 1536）、思考档位（`OPENAI_THINKING_LEVEL`）、token 用量统计、Apple 风格重设计。

## 关键决策与坑

- **上下文溢出三层修复**：工具输出统一 `capOutput` 40k；`trimHistory` 的 `shrinkToBudget` 保留 tool_call 配对；token 估算 CJK 1 字/token、ASCII ≈3 字符/token。详见 [llm-context-overflow](../30-Resources/troubleshooting/llm-context-overflow.md)。
- **spawn 子进程必须带 `PATH + HOME`**：`execFile` 的 env 是整体替换，丢了 HOME 后 aliyun CLI 找不到 `~/.aliyun`。详见 [aliyun-cli-oauth-sls](../30-Resources/troubleshooting/aliyun-cli-oauth-sls.md)。
- **CORS 预检**：集成测试用 fetch 不走 CORS，DELETE 漏配方法会「测试全绿、浏览器静默失败」。详见 [koa-server-gotchas](../30-Resources/troubleshooting/koa-server-gotchas.md)。
- **图表帧**：`tool.end` 帧会截断 2000 字，图表规格单独发 `chart` SSE 帧，前端 `ChatMessage.charts` 累积。
- **流式工具事件**：LangGraph 1.5 `streamMode` 无 `events`，从 `updates` 提取 model/tools 节点变化生成 `tool.start` / `tool.end`。

## 当前状态（2026-10-16 收盘）

- 10-16 12:35 按口令批量提交 6 commits 并推送（`cd608bf`…`29f9a7b`），13:05 代码导读补齐（`8f119ae`）。
- **未提交**：10-16 下午的思考档位 / 附件输入 / 上下文溢出修复 / 架构图等 22 files（+4 新文件），等口令；`docs/assets/{architecture,request-flow}.{mmd,png}` 未 commit。

## 日志索引

- 10-14：项目初始化（`/Users/colna/TEST/agent-demo`，GitHub `koa-agent`），Phase 1 骨架。
- 10-15：SLS skill 接入、思考/工具展示、体验闭环 + Apple 重设计、BytePlus、SP 双环境。
- 10-16：图表工具、SLS 双环境、RAG + token 统计、批量提交、架构图、思考档位 + 附件、上下文溢出修复。
