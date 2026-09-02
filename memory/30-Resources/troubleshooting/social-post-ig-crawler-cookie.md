---
title: social-post IG 爬虫 502/503「无法从主页 HTML 解析 user id」= IG_COOKIE 失效
tags: [social-post, instagram, crawler, troubleshooting]
---

# 现象
`POST social-post-server.vercel.app/api/accounts/<id>/crawl` 返回 **503**(NestJS 包装),body:
`crawler error 502: {"detail":"抓取上游失败: 无法从主页 HTML 解析 user id"}`。

# 根因
IG 现在对 `web_profile_info` 和主页 HTML 都要登录态。crawler 无有效 cookie 时:
- `GET /api/v1/users/web_profile_info/?username=<h>` → **401** `{"require_login":true,"message":"Please wait a few minutes..."}` → FetchError
- 回退抓主页 HTML → 拿到**登录墙外壳**(title 仅 "Instagram",无 `profilePage_`/`props.user.id`)→ `parse_profile_html` 返回 None → 抛「无法从主页 HTML 解析 user id」(`services/crawler/app/crawlers/instagram.py:178`)
- crawler 502 → NestJS `crawler.service.ts` `!resp.ok` → ServiceUnavailableException 503

cookie 来源:crawler 只吃 env `IG_COOKIE`(`services/crawler/app/config.py:17`)。NestJS `CrawlerService.crawl` 只发 `{handle,maxPosts,since,until}`,**不传 per-account cookie**。故 `IG_COOKIE` 失效即全线挂。

# 修法
- **立刻**:更新 crawler 部署的 `IG_COOKIE`(`sessionid`+`ds_user_id`+`csrftoken`)。验证 `curl web_profile_info -H "Cookie: ..." -H "x-ig-app-id: 936619743392459"` 应 200。(2026-09-01 已用此法修复,crawl 恢复 201。)
- 更稳:按账号存 cookie 走 crawler 已支持的 `body.cookie`(`fetch_profile(handle,opts,cookie)`),NestJS 补链路,避免单账号风控连坐。
- IP:Vercel 机房 IP 被 IG 标记 → 出口走住宅/移动代理或迁出 Vercel;`SCRAPLING_MODE=fetch` 无浏览器过不了挑战。

# 潜藏 bug(未修)
`parse_profile_html` 正则 `profilePage_\d+` / `"props":{"user":{"id":"`(`parsers/instagram.py:122-124`)是旧版 IG 标记,现登出 HTML 已不含 → 该回退路径即使 200 也失效,建议更新或废弃。
