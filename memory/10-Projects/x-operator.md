---
title: x-operator（Creator Change Log）
date: 2026-10-28
tags: [project, x, twitter, automation, content, monetization]
---

# x-operator（Creator Change Log）

## 目标

英文 X 账号，0 粉起步，全自动运营，面向欧美创作者/社媒运营。无 Premium、零预算；不做 X 广告分成，变现路径为创作者工具联盟 → newsletter → 赞助/数字产品。

## 现状（2026-10-28）

- 代码：`/Users/colna/WORK/x-operator`，约 1,263 行，4 个单测通过；尚未 `git init`/推送。
- 已实现：X API OAuth 1.0a 发帖、两阶段 pending 防重复、RSS + Google Trends 选题、OpenRouter 免费模型生成、24 条队列、每日指标、周报、GitHub Actions 三个 workflow。
- 已验证：本地 mock LLM 全链路；10 个候选、3 条有效、1 条被平台配额规则拦下；`npm run doctor` 能报缺 key。
- 阻塞：X 账号与开发者 key、OpenRouter key、GitHub 私有仓库创建/推送。
- 本地网络必须走系统代理 `http://127.0.0.1:7897`；GitHub Actions 不需要。

## 关键决策

- Niche：平台规则/功能变更 + 可执行动作，覆盖 Instagram、TikTok、YouTube、X、LinkedIn。
- 节奏：3 条/天（UTC 12:30 / 16:30 / 21:30），免费 API 额度内；每条附 source reply。
- 不做关注/私信/自动回复，避免封号和 automation 规则风险；账号打开 automated 标签。
- 内容优先级：新新闻 prepend 到队列头部；introduction 帖 `priority: 100` 先发。
- 停止线：90 天 <100 粉或 <5 万展示；180 天 <$50 收入。

## 架构

- `src/post.mjs`：`prepare` 写 pending 并提交 → `post` 发主帖 + source reply；失败/超时可恢复。
- `src/generate.mjs`：RSS/Google Trends → 相关性/来源黑名单/平台配额 → LLM → 校验 → prepend。
- `src/metrics.mjs`：每日 own timeline，按 tweet id 去重，周日生成周报。
- Workflows：`post.yml`、`generate.yml`、`metrics.yml`，均带 `contents: write` 和 concurrency。

## 风险

- 无人工互动 + automated 标签，增长慢，广告分成不可用。
- 联盟申请和收款 KYC 必须用户本人完成；自动账号可能被联盟拒绝。
- 纯 AI 内容有平台降权风险；当前用“标题事实 + 可执行动作”降低水文感，但仍需看 90 天数据。
