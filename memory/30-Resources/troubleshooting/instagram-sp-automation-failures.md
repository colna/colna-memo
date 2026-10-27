---
title: IG Social Proxy 批量任务失败（404 锁死 / 脚本版本 / 超时）
date: 2026-10-24
tags: [instagram, social-proxy, app-ins-scripts, troubleshooting]
---

# IG Social Proxy 批量任务失败

## 症状

批量 story/comment 任务大面积失败，错误码集中在：

- `PROFILE_NOT_FOUND`（「目标页面不存在 (404)」），`dom_snapshot.url` 往往不是本任务目标，而是上一个任务的页面
- `UNKNOWN_ACTION: 未知方法: commentPost`
- `"Cannot read properties of undefined (reading 'execute')"` / `"window.SocialProxy.execute is not a function"`
- `SESSION_LOST`（空白页）
- `Timeout: commentPost for <creatorId> (60000ms)`
- `STORY_PUBLISH_FAILED: 烧录接口失败：HTTP -1`、`Cannot read properties of undefined (reading 'importKey')`

## 根因

1. **404 页面自我锁死（放大器，最致命）**
   `SP.execute` / `SP.pipeline` 在执行任何非导航 action 前，先对**当前页面**调 `detectPageState()`（`packages/app-ins-scripts/src/instagram/actions/executeAction.js:185,410`）。页面 title 含 "not found" 就直接返回 `PROFILE_NOT_FOUND`，**不会导航到目标页**。于是设备踩到一次 404 后，后续每个任务的 `startStep=0` 前置检查都失败 → 整台设备全灭，日志里表现为几十条任务都停在同一个 404 URL。
2. **页面上下文不重置**
   同一设备串行跑多个 creator 的 story + comment，story 和 comment 之间也不重置页面；story 的报错 URL 会是 comment 任务的目标帖子页。
3. **脏 socialId**
   邮箱（`xxx@gmail.com`）、带空格显示名（`salem jafar`）被当 IG handle 拼进 URL，必然 404；格式合法但账号被删/改名/封禁同样 404。
4. **脚本版本不一致**
   设备上的 automation.js 没有 `commentPost` → `UNKNOWN_ACTION`；SP 对象结构旧 → `execute is not a function`。热更未覆盖到设备（order 29）。
5. **网络/代理**
   `chrome-error://chromewebdata/`（页面未加载）、`HTTP -1`（原生请求没发出去）、`importKey` undefined（非 http(s) 页面没有 `crypto.subtle`）、白页。
6. **默认 60s 超时太紧**
   `COMMENT_POST` 未配置 `timeout_func`，走默认 60s（`packages/app-social-proxy-server/src/v3/behavior/behavior.service.ts:488`）；单条评论「30s 等输入框 + 300ms/字符打字 + 1.5–2s 点击延迟」很容易超。

## 排查方法

- 按 `creator_id` 交叉看状态序列（不是按 shortCode）：同一 creator 反复失败在同一 URL = 设备页面锁死；失败在不同 URL = 数据/网络问题。
- 比对目标 `short_code` 与 `dom_snapshot.url` 里的 shortCode：不一致 → 前置检查拦截（页面污染），不是目标帖子 404。
- `UNKNOWN_ACTION` / `execute` 缺失统一归到脚本版本，查设备实际加载的 automation.js 版本与 `upload-all.mjs` 清单。

## 修法

1. 前置检查只拦 `logged_out` / `banned` / `white_page`；`not_found` 不阻断，改为强制导航到目标 URL；或检测到 404 时先导航 `/` 再继续。
2. 每个任务开始前显式导航到目标页，不复用当前页面；story 与 comment 分设备或分页面。
3. 入库/下发前校验 socialId：`^[A-Za-z0-9._]{1,30}$`，剔除邮箱和空格；404 的帖子/账号标记失效，停止派发。
4. 脚本热更加版本校验，缺失方法时明确报「脚本版本过低」并触发拉取。
5. `COMMENT_POST` 显式配置 120s 以上 timeout，或缩短等输入框时间。
6. 任务队列按设备吞吐排期，避免 60s/条的串行任务堆积到过期（CANCELLED）。

## 2026-10-26 12 条 IG 发帖失败归因补充

- **`PROFILE_NOT_FOUND`（7 条）**：`createPost` 不访问目标主页，正常不可能由它返回。更可能是设备仍在跑旧版 `executeAction`，被上一任务残留的 404 页面前置检查锁死。`createPost` 已在 `SESSION_ONLY_ACTIONS`，执行前只应校验会话。
- **`VIDEO_INIT_FAILED`（2 条）**：唯一来源是 `createPost.js` 的 `initVideoUpload`，请求 `GET i.instagram.com/rupload_igvideo/...`。`302` 通常是请求未被识别为已登录（cookie/sessionid 缺失或失效）；`403` 通常表示会话存在但被风控或限制上传。不要先归因于素材或上传代码。
- **`hasActiveSession()` 的盲区**：只检查 `ds_user_id` cookie 是否存在，无法识别「cookie 还在、sessionid 已死」，会把失效账号放行到真正发请求时才失败。
- **`execute is not a function`**：Android `ActionDispatcher` 直接调用 `window.SocialProxy.execute(...)`；设备 automation 未含 `executeAction`（热更或注入缺失），客户端也没有 `typeof` 防御。
- **下一次取证**：同时收集完整 `error_detail`、`dom_snapshot.url`、设备实际加载的 automation 脚本版本，再区分页面污染、账号失效与 IG 风控。
