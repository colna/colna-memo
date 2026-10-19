---
title: LLM Agent 工具输出撑爆上下文（400 max context）与三层修复
date: 2026-10-19
tags: [llm, agent, langgraph, context, token, troubleshooting]
---

# LLM Agent 工具输出撑爆上下文（400 max context）

来源：`/Users/colna/TEST/social-proxy-agent`（2026-10-16）。姊妹条目：[langgraph-js-streaming](langgraph-js-streaming.md)（流式协议）。

## 现象

流式对话报 400：`maximum context length is 1048576, requested 2835009`。

## 根因（查 sqlite 实证）

- `sp_dashboard_query` 的 trace（prod）返回 **2–3.5MB 原始 JSON**，无截断；同一轮 3 个调用共约 6.5M 字符。
- `trimHistory` 的「单轮超预算保整轮」兜底把超限内容原样发给模型，等于没有兜底。

## 修法（三层，缺一不可）

1. **工具层封顶**：所有外部数据工具输出统一走 `capOutput`（40k 截断 + 「缩小范围」提示）；SLS 这类可能单行超长的输出也要封顶。
2. **裁剪层兜底**：`trimHistory` 增加 `shrinkToBudget`——把最重的 `ToolMessage` 文本逐步截半（保留 `tool_call_id` 配对），`HumanMessage` 先丢图片再截文；**绝不动含 `tool_calls` 的 `AIMessage`**，否则工具调用配对断裂。
3. **估算校准**：CJK 按 1 字/token，ASCII/JSON 约 3 字符/token（原按 4 字符/token 会低估 JSON；实测 ≈2.3 字符/token），图片块按 base64 长度/3 计。

## 验证

真实故障 thread `t_faab98b1f1174edf`：修复前估算 1.64M → 收缩后 41.7k（预算 48k）；复跑成功（done 帧，无 400）。

## 教训

- 外部 API 的输出体积不可信，**必须在工具层封顶**，不能只靠上下文裁剪。
- 裁剪兜底要么保证结构合法（tool_call 配对），要么宁可报错；半吊子截断会让模型收到坏消息链。
- token 预算按字符类型估算；中文日志按「长度/4」会低估约 3 倍（48k 预算真实 input 达 79k）。
