---
title: Onia
date: 2026-11-19
tags: project, onia, social-proxy, architecture, rust, flutter, monorepo, agent, protocol
---

# Onia

个人项目的 Social Proxy（SP）应用。管线继承 Signal（事实）→ Todo（决策）→ Behavior（执行），协议与模块按层拆分、职责独立。服务端 Rust，业务主库 PostgreSQL，含 Agent（用户 AI 自动回复）、Web 官网、Web 后台和脚本，全部放在同一个私有 monorepo。

- 技术架构：[architecture.md](architecture.md)
- 文档只写在 colna-memo，不使用飞书。
- 当前进展：[status.md](status.md)
- 功能与技术现状梳理（独立成稿）：[onia-status-overview.md](onia-status-overview.md)
- 竞品调研与商业模式建议：[onia-business-model.md](onia-business-model.md)
- 官方 API 入站触发技术方案：[onia-official-api-inbound.md](onia-official-api-inbound.md)
- 商业化方案（收费、功能拆分与 AI 成本）：[onia-commercialization.md](onia-commercialization.md)
- Instagram 动作与 Pipeline 方案：[instagram-actions-pipeline.md](instagram-actions-pipeline.md)
- Signal 多动作串行与编辑页（产品口径）：[signal-multi-action.md](signal-multi-action.md)
- Signal 多动作串行与编辑页（实现方案）：[signal-multi-action-impl.md](signal-multi-action-impl.md)
- Instagram 聊天记录与新消息监听接入：[instagram-dm-sync.md](instagram-dm-sync.md)
- Instagram 收到消息后自动回复（阶段 C）：[instagram-auto-reply.md](instagram-auto-reply.md)
