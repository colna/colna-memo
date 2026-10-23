---
title: social-post-agent
date: 2026-10-19
tags: project, agent, social-post, koa, langgraph, nextjs
---

# social-post-agent

`social-post` 的对话式 Agent 版本：自然语言管理社媒账户、抓取 Instagram / Facebook 帖子、查询本地数据；FB 可服务端直连（fbCookie），浏览器扩展作为稳定兜底。参考 `social-proxy-agent` 架构。

## 基本信息

- 路径：`/Users/colna/WORK/social-post-agent`
- 技术栈：Koa 3 + zod + LangGraph JS（OpenAI 兼容，默认 DeepSeek）+ next 16 + React 19 + SQLite（better-sqlite3）
- 启动：`pnpm install && pnpm dev`；web `http://127.0.0.1:3001`，server `http://127.0.0.1:3000`
- 数据：`apps/server/data/`（`social-post.sqlite` / `agent.sqlite` / `settings.json`，均 gitignore）
- 文档：`docs/01~04` + README

## 能力

- Agent 工具：`social_list_platforms` / `social_list_accounts` / `social_add_account` / `social_crawl_account` / `social_list_posts` / `social_remove_account`（两步确认）+ `render_chart`
- Instagram：Node 直连 `web_profile_info` → 主页 HTML 回退 → user feed 翻页；可配置原 Python crawler
- Facebook：`fbCookie` → 主页 HTML / GraphQL（首屏 + 少量翻页）；`POST /ingest/facebook` + `x-ingest-token` 仍为稳定兜底；Chrome MV3 扩展（`extension/`）单页/批量/滚动采集，Tampermonkey 脚本备选
- 帖子表格：`social_list_posts.markdownTable` 固定 8 列（封面/类型/文案/点赞/评论/转发/发布时间/原帖），web 表格内图片 56px 缩略图
- Web：聊天 SSE、设置弹窗（模型 / IG / FB / crawler，GET 脱敏）、数据抽屉（账户/帖子/抓取/删除/搜索/分页）
- 设置保存后 revision +1，Agent Runtime 重建模型图，无需重启

## 关键决策

- **本地优先**：SQLite + JSON，不依赖 Postgres / Python；外部 crawler 是可选增强。
- **凭证运行期可改**：所有 token 可在网页填，settings.json 权限 0600，GET 只返回 hint。
- **Facebook 服务端直连可选**：配置 fbCookie 后走主页 GraphQL，限制单次条数与翻页次数；失败降级内嵌帖子。高频/稳定采集仍用浏览器扩展 ingest，避免 1357054 软封。
- **删除账户两步确认**：第一次 `requiresConfirm`，第二次 `confirm=true`。
- **模型无 Key 可启动**：聊天入口返回 503 配置提示，线程/设置/数据接口仍可用。

## 状态（2026-10-23 17:15）

- 测试：server 105 + web 27 = 132 passed；lint / typecheck / build 全过。
- 新增：FB 服务端直连（`facebook.ts` / `facebook-parse.ts`）、Agent 帖子 8 列表格、undici 代理（HTTPS_PROXY/HTTP_PROXY）。
- 实机 smoke：agent 对话抓取 FB NASA 4 条帖子，输出 8 列表格（封面图片 / 类型 / 文案 / 点赞 / 评论 / 转发 / 时间 / 原帖）。
- 本地 commit：`85ae4b9`、`cc0bb60`、`dd70925`、`a2f9a0c`、`4fef129`、`6afdbc2`、`0e7362a`、`6e7bf53`、`c322a1a`。
- 未配置 GitHub remote；未做线上部署。

## 修复记录

- 11:05：CORS 预检漏 PUT 导致设置保存被拦截；SqliteSaver 懒建表导致空库 `/agent/threads` 500。均已修复并加回归测试。

## 已知限制

- IG 直连未做 TLS 指纹伪装，高频抓取可能触发 429 / 登录墙；建议配置原 crawler。
- 无登录/多租户，只适合本机使用。
- FB 服务端直连依赖 fbCookie，可能触发风控/软封；扩展与 FB 实机采集仍需用户装扩展后验证。
