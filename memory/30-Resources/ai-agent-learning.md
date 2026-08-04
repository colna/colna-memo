---
title: 学做 AI Agent — 路线与教材
date: 2026-08-04
tags: [agent, llm, 学习, reference]
---

# 学做 AI Agent — 路线与教材

## 核心概念
- **LLM = 大脑**(文字进文字出,无状态、不能动手);**Agent = LLM + 工具 + 循环 + 记忆**。
- 机制:**tool use / function calling** —— 模型输出「调哪个工具+参数」,agent 框架真执行再回喂,循环到完成。
- 公式(与李博杰书一致):**Agent = LLM + Context + Tools**。

## 主教材(强烈推荐)
- **《AI Agents in Depth》李博杰** — https://github.com/bojieli/ai-agent-book
  - 免费开源(Apache 2.0)、有中文版、10 章 + **95 个可跑实验**(Python 3.10+),支持 OpenAI/Claude/**DeepSeek/Kimi/GLM** key。
  - 章节:1 基础 / 2 Context Engineering(KV cache/压缩/skills)/ 3 Memory & RAG/知识图谱 / 4 Tools(MCP)/ 5 Coding Agents / 6 评测 / 7 后训练 SFT·RL / 8 持续进化 / 9 多模态·GUI·机器人 / 10 Multi-Agent。
  - **面向「搭一个能用的 agent」读法**:Ch1→4→2/3→5→6;先跳 Ch7/Ch9。

## 学习路线(动手为主,先手写再上框架)
0. 打通 LLM API(messages/token/tool use 概念)。
1. **手写最小 agent 循环 ~150 行,2-3 工具,不用框架**(最关键,别跳)。
2. 真实工具 + **MCP**(shell/文件/HTTP)。
3. 上下文 + 记忆 + **RAG**(向量检索)。
4. 规划/ReAct/反思/护栏,再上框架(LangGraph / Vercel AI SDK),读开源 agent 源码(Aider/OpenHands)。
5. 界面(CLI→飞书→web)、评测、多 agent。
- 必读:Anthropic《Building Effective Agents》、ReAct 论文、tool-use 文档。
- 栈:学得快用 **Python**;贴 TS 团队用 **Vercel AI SDK / MCP TS SDK**。

## 相关横向对比
- **Claude Code / MetaBot**(我):Anthropic Claude 内核,MetaBot=Claude Code 套飞书壳+编排,深接工程闭环。
- **GitHub Copilot coding agent**:云端、GitHub 原生、开 PR 走 CI;模型限 GitHub 启用的 GPT/Claude/Gemini。
- **WorkBuddy(腾讯 CodeBuddy)**:桌面 AI Agent 工作台(办公+代码+设计),skills+MCP,国内版可切 DeepSeek/GLM/Kimi/MiniMax。
