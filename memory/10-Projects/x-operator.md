---
title: x-operator（Creator Change Log）
date: 2026-10-28
tags: [project, x, twitter, automation, content, monetization]
---

# x-operator（Colna | Creator Change Log）

## 目标

英文 X 个人 IP 账号 Colna，主打 Creator Change Log 系列，0 粉起步，全自动运营，面向欧美创作者/社媒运营。个人是脸和语气，Creator Change Log 是内容主线。无 Premium；X API 为 pay-per-use，最低买一次 credits 拿 $20 赠送额度，90 天测试约 $3/月。不做 X 广告分成，变现路径为创作者工具联盟 → newsletter → 赞助/数字产品。

## 现状（2026-10-29）

- 账号：已有 `@colna_zheng`（0 粉、1 following、2025-12 注册），等同新号；handle 保留，显示名 `Colna`，bio 已设 Creator Change Log，头像已换。公开 bio 不写 AI/自动化；不编造亲测经历/客户案例/数据。
- 代码：`/Users/colna/WORK/x-operator`；私有仓库 `https://github.com/colna/x-operator` 已推送，main 已同步；4 个单测通过；最新 commit `f4175ef`。
- 素材：`assets/avatar.png`、`assets/banner.png`、`assets/banner-generic*.png`；配图已定稿手绘明亮风，活跃素材 `assets/generated/instagram-90min-handdrawn.png`、`youtube-payout-handdrawn.png`、`tiktok-shop-handdrawn.png`；旧深色 AI 图保留但不再使用。profile banner 仍为空，X API 不能改，需手动上传。
- 已实现：OAuth 2.0 Bearer 发帖 + refresh 加密状态、两阶段 pending 防重复、Humanizer 二遍编辑、RSS + Google Trends 选题、DeepSeek 生成、每日指标、周报、GitHub Actions；新增 AIGC Studio 手绘配图自动生成（`src/lib/post-image.mjs`；`src/post.mjs` 支持 `image_path` 或 `ENABLE_IMAGES=true` 自动生成+上传；`post.yml` 已注入 AIGC env）；`image-test` workflow 在 Actions 验证成功（2.5MB 图，约 28s）。
- 已验证：首条 `tweet 2115381703002947668`；手绘 Instagram 重发 `tweet 2115634640224706688`（已确认 `media_keys` 有附件）；旧 YouTube/Instagram 平铺帖已删除；YouTube payout 改写（`priority:40` + 手绘图）排队；TikTok Shop 已挂手绘图；`doctor` x_auth true + oauth2_refresh true；`npm test` 4/4。
- 上线状态：自动化已上线；2026-10-29 已发 1 条手绘 Instagram，旧帖删除。下一条定时帖为 2026-10-30 12:30 UTC（20:30 北京）的 YouTube payout 改写（带手绘图）；之后 TikTok Shop（带图），其余无 `image_path` 的帖子自动生成手绘图。剩 banner 未上传、$20 赠送未到账；DeepSeek key 建议轮换。
- 配图 cookie：`~/.aigc_studio_creds`（600）2026-11-04 过期；已设 GitHub secret `AIGC_STUDIO_COOKIE`，vars `AIGC_STUDIO_BASE=https://aigc-studio.sitinai.com`、`ENABLE_IMAGES=true`。过期后自动配图会失败并退回纯文本，需重新复制 `aigc_session` 更新 secret；更新后跑 `image-test` 验证。
- 成本：Post $0.015/次，带 URL $0.20/次（默认不发链接）；Posts Read $0.005/resource；4 发 + reads 后余额约 $4.9，$20 赠送未到账。
- OAuth state 注意：GitHub workflows 会轮换 refresh token，本地 `state/oauth2.enc.json` 可能过期；操作前先 `git pull --rebase --autostash`，以 state 为准，不要用本地 `.env` 的旧 token。
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
- 配图：2026-10-29 定稿手绘明亮风（暖米色纸、黑手绘线、珊瑚红/青绿/芥末黄、无文字/logo/真人）；prompt 在 `content/image-prompt.txt` / `content/image-style.md`，AIGC `gpt-image-2.5-sunburst` 3:2；不再用深色 AI 图。
- 内容优先级：新新闻 prepend 到队列头部；introduction 帖 `priority: 100` 先发。
- 个人 IP：第一人称、短句、观点明确；不冒充亲测，不写成品牌公告。
- 停止线：90 天 <100 粉或 <5 万展示；180 天 <$50 收入。

## 上线状态

- 首条：`https://x.com/colna_zheng/status/2115381703002947668`
- 第二条（手绘 Instagram）：`https://x.com/colna_zheng/status/2115634640224706688`
- 旧 YouTube `2115403860965781914`、旧 Instagram `2115420088350478660` 已删除。
- 仓库：`https://github.com/colna/x-operator`（private，commit `f4175ef`）
- 定时：generate 06:00 UTC；post 12:30 / 16:30 / 21:30 UTC；metrics 23:00 UTC
- 今天已发 1 条（手绘 Instagram）；12:30 UTC cron 因 `daily_limit` 跳过；明天恢复 1 条/天，预计 2026-10-30 12:30 UTC 发 YouTube payout 改写（带手绘图）。
- 余额：约 $4.9；$20 赠送未到账
- 风险：控制台生成的 OAuth 2.0 token 带 DM scope；代码不调用 DM，但 token 权限较大，必须只存 `.env`/secret/加密 state。

## 架构

- `src/post.mjs`：`prepare` 写 pending 并提交 → `post` 发主帖（可选 source reply）；失败/超时可恢复。
- `src/generate.mjs`：RSS/Google Trends → 相关性/来源黑名单/平台配额 → LLM（`source_index` 映射 source_url，避免长 URL 截断）→ Humanizer 二遍编辑 → 校验 → prepend。
- `src/lib/oauth2.mjs`：refresh token 轮换 + AES-256-GCM 加密状态；`ensureOAuth2Token` 在 post/metrics/doctor 前执行。
- `src/lib/x.mjs`：`uploadMediaV2`（v2 JSON base64）+ `createTweetV2`（undici，绕开代理下 `twitter-api-v2` 的 TLS reset）；`post.mjs` 支持 `image_path` 附件。
- `src/lib/post-image.mjs`：AIGC Studio 生成 3:2 手绘图（创建→轮询→下载），`post.mjs` 对无 `image_path` 的帖子自动生成并上传；`image-test.mjs` / `image-test.yml` 只验证生成、不发帖。
- 配图：image2 网关 `GROUP_NOT_ALLOWED`（区域封锁）；改走 AIGC Studio `POST /api/generations`（cookie），模型 `gpt-image-2.5-sunburst`，输出 1536x1024 PNG。`image2-gen` skill 已改为默认 AIGC Studio/Sunburst，旧网关用 `--provider api`。
- `src/metrics.mjs`：每日 own timeline，按 tweet id 去重，周日生成周报。
- Workflows：`post.yml`、`generate.yml`、`metrics.yml`，均带 `contents: write` 和 concurrency。

## 风险

- 无人工互动 + automated 标签，增长慢，广告分成不可用。
- 联盟申请和收款 KYC 必须用户本人完成；自动账号可能被联盟拒绝。
- 纯 AI 内容有平台降权风险；当前用“标题事实 + 可执行动作”降低水文感，但仍需看 90 天数据。
