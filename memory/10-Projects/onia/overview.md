---
title: Onia
date: 2026-11-11
tags: project, onia, social-proxy, architecture, rust, flutter, monorepo, agent, protocol
---

# Onia

个人项目的 Social Proxy（SP）应用。管线继承 Signal（事实）→ Todo（决策）→ Behavior（执行），协议与模块按层拆分、职责独立。服务端 Rust，业务主库 PostgreSQL，含 Agent（用户 AI 自动回复）、Web 官网、Web 后台和脚本，全部放在同一个私有 monorepo。

- 技术架构：[architecture.md](architecture.md)
- 文档只写在 colna-memo，不使用飞书。
- 当前进展：[status.md](status.md)
