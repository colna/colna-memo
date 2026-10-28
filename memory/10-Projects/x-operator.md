---
title: x-operator（Creator Change Log）
date: 2026-10-28
tags: [project, x, twitter, automation, content, monetization]
---

# x-operator（Colna | Creator Change Log）

## 目标

英文 X 个人 IP 账号 Colna，主打 Creator Change Log 系列，0 粉起步，全自动运营，面向欧美创作者/社媒运营。个人是脸和语气，Creator Change Log 是内容主线。无 Premium；X API 为 pay-per-use，最低买一次 credits 拿 $20 赠送额度，90 天测试约 $3/月。不做 X 广告分成，变现路径为创作者工具联盟 → newsletter → 赞助/数字产品。

## 现状（2026-10-28）

- 账号：已有 `@colna_zheng`（0 粉、1 following、2025-12 注册），等同新号；handle 保留，显示名 `Colna`，bio 已设 Creator Change Log，头像已换。公开 bio 不写 AI/自动化；不编造亲测经历/客户案例/数据。
- 代码：`/Users/colna/WORK/x-operator`，约 1,300 行，4 个单测通过；尚未 `git init`/推送。
- 素材：`assets/avatar.png`、`assets/banner.png` 已生成，待上传；头像先用 C 标，个人 IP 最好换真人照片。
- 已实现：OAuth 2.0 Bearer 发帖 + refresh 加密状态、两阶段 pending 防重复、Humanizer 二遍编辑、RSS + Google Trends 选题、DeepSeek 生成、24 条队列、每日指标、周报、GitHub Actions 三个 workflow。
- 已验证：首条已发 `tweet 2115381703002947668`；`doctor` x_auth true + oauth2_refresh true；`npm test` 4/4；`DRY_RUN=1 node src/generate.mjs` 生成 4 条 humanized posts。
- 阻塞：GitHub 私有仓库创建/推送 + secrets，等「推」；banner 未上传；$20 赠送未到账。
- 成本：Post $0.015/次，带 URL $0.20/次（默认不发链接）；Posts Read $0.005/resource，指标每天 10 条；约 $2.85/月，$20 赠送额度 3 个月过期。首条已花 $0.015，余额约 $4.985。
- 本地网络必须走系统代理 `http://127.0.0.1:7897`；GitHub Actions 不需要。

## 关键决策

- Niche：平台规则/功能变更 + 个人 operator take，覆盖 Instagram、TikTok、YouTube、X、LinkedIn。
- 节奏：3 条/天（UTC 12:30 / 16:30 / 21:30），免费 API 额度内；默认无 source reply，`ENABLE_SOURCE_LINKS=true` 才附。
- 不做关注/私信/自动回复，避免封号和 automation 规则风险；个人 IP 路线不公开 AI/自动化，平台仍可能自动标注。
- 只用官方 API，不用浏览器/登录会话自动化。
- 不走 X web 逆向/会话自动化：违反 ToS、绕过 pay-per-use、封号风险；REA 只用于合法分析。
- OAuth 1.0a 反复 401，已切 OAuth 2.0；Client ID 实际是 `alF4...`（小写 l，不是数字 1），refresh token 6 个月，状态用 AES-256-GCM 加密存 `state/oauth2.enc.json`，key 为 `OAUTH2_STATE_KEY`。
- LLM 用 DeepSeek；Humanizer skill 已装 `~/.agents/skills/humanizer`，生成后二遍编辑，`HUMANIZER=off` 可关。DeepSeek key 不落仓库（已在聊天暴露，建议轮换）。
- 风控：预热 1→2→3 条/天；默认不发链接；0-10 分钟 jitter；资料完整后再发；开发者 app 不勾 DM。
- 内容优先级：新新闻 prepend 到队列头部；introduction 帖 `priority: 100` 先发。
- 个人 IP：第一人称、短句、观点明确；不冒充亲测，不写成品牌公告。
- 停止线：90 天 <100 粉或 <5 万展示；180 天 <$50 收入。
- 风险：控制台生成的 OAuth 2.0 token 带 DM scope；代码不调用 DM，但 token 权限较大，必须只存 `.env`/secret/加密 state。

## 架构

- `src/post.mjs`：`prepare` 写 pending 并提交 → `post` 发主帖（可选 source reply）；失败/超时可恢复。
- `src/generate.mjs`：RSS/Google Trends → 相关性/来源黑名单/平台配额 → LLM（`source_index` 映射 source_url，避免长 URL 截断）→ Humanizer 二遍编辑 → 校验 → prepend。
- `src/lib/oauth2.mjs`：refresh token 轮换 + AES-256-GCM 加密状态；`ensureOAuth2Token` 在 post/metrics/doctor 前执行。
- `src/metrics.mjs`：每日 own timeline，按 tweet id 去重，周日生成周报。
- Workflows：`post.yml`、`generate.yml`、`metrics.yml`，均带 `contents: write` 和 concurrency。

## 风险

- 无人工互动 + automated 标签，增长慢，广告分成不可用。
- 联盟申请和收款 KYC 必须用户本人完成；自动账号可能被联盟拒绝。
- 纯 AI 内容有平台降权风险；当前用“标题事实 + 可执行动作”降低水文感，但仍需看 90 天数据。
