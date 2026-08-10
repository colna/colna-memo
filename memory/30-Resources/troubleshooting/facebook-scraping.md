---
title: Facebook 主页/帖子抓取(cookie + www + GraphQL)
date: 2026-08-10
tags: [troubleshooting, facebook, scraping, crawler]
---

# Facebook 抓取踩坑与可行路径

场景:social-post 采集平台加 FB,抓某主页(Page)的资料 + 帖子。2026-08-10 实测(带真实登录 cookie)。

## 结论速查

- **必须带已登录 fb cookie**(整段 `cookie:` 头,含 `c_user`/`xs`/`datr`)。无 cookie 三入口全 400/302 跳登录。
- **mbasic.facebook.com 已死**:带 cookie 也返回「错误」页(FB 下线中),别再用。
- **m.facebook.com**:现在是 React SPA 空壳(~41KB),无有用数据。
- **桌面 www.facebook.com/<handle>/**:GET 可用且**不触发反自动化**,但:
  - 资料:名称 / 主页数字 id / 主页链接 能拿;**粉丝数、帖子数不在初始页**(懒加载)。
  - 帖子:页面只内嵌**约 1 条**(在 `<script type="application/json">` 的 `data.user.timeline_list_feed_units.edges`)。
  - GET 必须带全套浏览器头(`Accept` / `sec-ch-ua` / `Sec-Fetch-*` / `Upgrade-Insecure-Requests`),否则 400。

## 拿多帖:GraphQL 翻页(技术可行但会被风控)

1. 从主页 HTML 抠:`"DTSGInitialData",[],{"token":...}`(fb_dtsg)、`"LSD",[],{"token":...}`、`"__spin_r"`、`"__spin_t"`、`"__spin_b"`、`"haste_session"`、`"hsi"`。
2. 从 `"preloaderID":"adp_ProfileCometTimelineFeedQueryRelayPreloader..."` 块抠 `"queryID"`(=doc_id,如 `27002668536074808`)+ `"variables"`(**按 `userID` 翻页**,不是 base64 id;含一堆 `__relay_internal__pv__*` provider 开关)。
3. POST `https://www.facebook.com/api/graphql/`,form-urlencoded,含上面所有参数 + `fb_api_req_friendly_name=ProfileCometTimelineFeedQuery` + `fb_api_caller_class=RelayModern` + `jazoest`("2"+fb_dtsg 各字符 ord 求和) + `doc_id` + `variables`(改大 `count`、翻页加 `cursor`)。header 带 `X-FB-LSD` / `X-FB-Friendly-Name` / `Origin` / `Referer`。
4. 响应是**流式多段**(每帖一段,`page_info.end_cursor` + `has_next_page` 在后段)。递归找 `__typename=="Story"` 且带 `post_id` 的节点即帖子。

### 致命坑

- **urllib/requests 发 GraphQL 直接被拒 `{"error":1357054,...}`** —— 真因是 **Python 的 TLS/HTTP2 指纹**。必须用 **`curl_cffi`(`impersonate="chrome"`)或 curl `--http2`**(Chrome 指纹)才成功。这也是 IG 爬虫用 curl_cffi 的原因。
- **即便指纹对、token 新鲜,连发 1~2 次 GraphQL 后 FB 就软封整个会话/账号**(后续含全新页面 token 的请求也持续 `1357054`)。→ 单纯 raw HTTP 无法稳定翻多页。稳定多帖需 **会话/住宅代理轮换** 或 **无头浏览器(Playwright)+ 拟人节流**,有封号风险。

### 帖子字段路径(Comet JSON,深且多变 → 递归按 key 搜,别写死)

- `post_id` / `permalink_url` / `creation_time`(node 顶层)
- 文案:所有 `message.text` 里最长的一条
- 点赞:`reaction_count.count`(取 max)
- 评论:`total_comment_count` 或 `comments.total_count`(常因关评为空)
- 封面:attachments 里第一张 `uri` 含 `fbcdn`
- 类型:styles `__typename` 含 `Album`→carousel;`is_playable`/`playable_url`/`Video`→video;否则 image

## 工程落地(social-post)

采用「www 主页解析(稳)+ 尽力 GraphQL 翻页 + 软封优雅降级为仅内嵌帖」。见 `social-post/services/crawler/app/{parsers,crawlers}/facebook.py`、docs/06。

## 附:本机 Python 环境坑

homebrew Python **3.14.6 坏了**(`pyexpat` 符号 `_XML_SetAllocTrackerActivationThreshold` 缺失,pip/ensurepip/venv 全崩)。跑 crawler pytest 用 **`uv venv --python 3.12`** 建干净环境。本工作区跑 Python 测试一律走 uv。
