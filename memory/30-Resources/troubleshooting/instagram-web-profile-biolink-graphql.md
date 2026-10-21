---
title: Instagram Web 通过 GraphQL 获取 profile bio link
date: 2026-10-21
tags: troubleshooting, instagram, graphql, profile, biolink, web
---

# Instagram Web 通过 GraphQL 获取 profile bio link

**结论**：IG Web 没有可直接按 username 拿 bio link 的单步接口；需要两步 GraphQL：

1. `PolarisSearchBoxRefetchableQuery`（doc `27706427925724183`）username -> pk
2. `PolarisProfilePageContentQuery`（doc `28717443601247930`）pk -> `data.user.bio_links[]`

## 1. username -> pk

`POST https://www.instagram.com/api/graphql`

表单关键字段：

- `fb_api_req_friendly_name=PolarisSearchBoxRefetchableQuery`
- `doc_id=27706427925724183`
- `variables={"data":{"context":"blended","include_reel":"true","query":"<username>","rank_token":"","search_session_id":"<uuid>","search_surface":"web_top_search"},"hasQuery":true}`
- 其余字段复用页面真实请求：`av`、`__d=www`、`__user=0`、`__a=1`、`__req`、`__hs`、`dpr=2`、`__ccg`、`__rev`、`__s`、`__hsi`、`__dyn`、`__csr`、`__hsdp`、`__sjsp`、`__comet_req=7`、`fb_dtsg`、`jazoest`、`lsd`、`__spin_r`、`__spin_b`、`__spin_t`、`__jssesw=1`、`fb_api_caller_class=RelayModern`、`server_timestamps=true`

请求头：

- `content-type: application/x-www-form-urlencoded`
- `x-csrftoken: <csrftoken cookie>`
- `x-ig-app-id: 936619743392459`
- `x-fb-lsd: <lsd>`
- `x-fb-friendly-name: PolarisSearchBoxRefetchableQuery`
- `referer: https://www.instagram.com/explore/search/`

响应：

`data.xdt_api__v1__fbsearch__topsearch_connection.users[0].user.pk`

搜索是模糊匹配，取 pk 前必须校验 `user.username === username`，防止同名/近似名。

## 2. pk -> bio links

同一 endpoint，改为：

- `fb_api_req_friendly_name=PolarisProfilePageContentQuery`
- `doc_id=28717443601247930`
- `variables={"id":"<pk>", ...页面原有的 __relay_internal__pv__* 字段}`

响应：

- `data.user.bio_links[]`：每项有 `url`、`lynx_url`、`title`、`link_type`
- `data.user.external_url`：老字段，可能为空字符串

**取值规则**：

- `url` 是真实目标，例如 `https://wa.me/48888800000`
- `lynx_url` 是 `https://l.instagram.com/?u=...` 跳转包装，带 `utm_source=ig&utm_medium=social&utm_content=link_in_bio`，不要直接当外链
- `bio_links` 可能 0~5 条，按需取第一条非空 `url` 或全部返回
- 不要只依赖 `external_url`，很多账号（含 shangbin2026）该字段为空

## 实测（2026-10-21，sessionid 登录态）

- `shangbin2026` -> pk `43150596056` -> `bio_links[0].url = https://wa.me/48888800000`（title `follow me22`），`external_url = ""`
- `nasa` -> pk `528817151` -> 5 条 bio_links，第一条 `https://www.nasa.gov`
- `xlc7637` -> pk `76449409072` -> `bio_links = []`

## 不进入 profile 页

2026-10-21 复测：停留在 `/direct/inbox/`，抓当前页任意一次 `/api/graphql` 请求作为模板（复用 `csrftoken`/`lsd`/`fb_dtsg`/`__dyn`/`__csr`/`__hsdp`/`__sjsp`/`av` 等字段），再按上面的两步 doc id 调用，**无需导航到 profile 页**，`shangbin2026` 同样返回 `https://wa.me/48888800000`。

实现要点：在页面里 monkey-patch `fetch`，缓存最近一次 GraphQL 的 headers + form body；调用时 clone 这份 body，只替换 `fb_api_req_friendly_name`、`doc_id`、`variables`、`__req`。`PolarisProfilePageContentQuery` 的 `__relay_internal__pv__*` 可沿用模板/硬编码值。

## 不可用 / 踩坑

- `GET /api/v1/users/web_profile_info/?username=...` -> 429（返回 HTML Page Not Found）
- `GET https://i.instagram.com/api/v1/users/{id}/info/` -> 403
- `GET /api/v1/web/search/topsearch/` -> 404
- 直接把 username 塞进 profile query 的 `id` -> GraphQL `field_exception`
- 只带 sessionid cookie 直连服务端不够，GraphQL 还需要 csrftoken + `lsd`/`fb_dtsg` 等页面 token；会 429/403

## 调用位置建议

- **最稳**：在 WebView/页面上下文里 `fetch('/api/graphql')`，Cookie 和 CSRF 自动带上；从页面读取 `csrftoken`、`lsd`、`fb_dtsg` 等字段。
- **服务端直连**：需要维护 sessionid + csrftoken + 整套 GraphQL 表单 token，且 token 会随页面刷新变化；不如 WebView 内调用稳。
- 两步请求都走 `https://www.instagram.com/api/graphql`，不要用 `i.instagram.com` 私有接口。

## 无模板兜底（页面加载后再注入）

2026-10-21 实测：页面加载完再注入脚本，`interceptUserInfo` 没有捕获到 GraphQL 模板，仍可直接从页面模块生成请求：

- `DTSG.getToken()` -> `fb_dtsg`
- `LSD.token` -> `lsd`
- `SiteData.haste_session` -> `__hs`
- `SiteData.client_revision` -> `__rev`
- `SiteData.comet_env` -> `__comet_req`
- `SiteData.__spin_r` / `__spin_b` / `__spin_t` -> 同名表单字段
- `CurrentUser.getPossiblyNonFacebookUserID()` -> `av`
- `CurrentUser.getAppID()` -> `x-ig-app-id`
- `jazoest = "2" + sum(charCodes(fb_dtsg))`

最小 body 可省略 `__dyn` / `__csr` / `__hsdp` / `__sjsp` / `__s`；实测 search + profile 都返回 200。脚本优先用模板，模板不存在时走这条 token 兜底，因此不依赖注入时机。
