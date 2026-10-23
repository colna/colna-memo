---
title: Instagram Web 主页帖子 GraphQL 逆向（2026-10）
date: 2026-10-23
tags: [troubleshooting, instagram, graphql, scraping, web]
---

# Instagram Web 主页帖子 GraphQL 逆向

2026-10-23 用已登录 cookie 实测 NASA 主页，替代/补充旧的 `/api/v1/feed/user/<id>/`。

## 结论速查

- endpoint：`POST https://www.instagram.com/api/graphql`
- 翻页 query：`PolarisProfilePostsTabContentQuery_connection`，`doc_id=28544666141895803`
- 首屏 query（可选）：`PolarisProfilePostsQuery`，`doc_id=29156678094015645`
- 必须带登录 cookie（至少 `sessionid`、`csrftoken`、`ds_user_id`）与 `X-IG-App-ID: 936619743392459`
- token 从主页 HTML 抓，不依赖浏览器：
  - `"DTSGInitialData",[],{"token":"..."}` → `fb_dtsg`
  - `"LSD",[],{"token":"..."}` → `lsd`
  - `"csrf_token":"..."` 或 cookie `csrftoken`
  - `"X-IG-App-ID":"..."`
  - `"qeid":"<viewer_pk>"` → 表单里的 `av`（viewer 内部 id，不是 `ds_user_id`）
- `jazoest = "2" + sum(charCode(fb_dtsg))`

## 请求形状

Headers（关键）：

```text
content-type: application/x-www-form-urlencoded
x-csrftoken: <csrf>
x-fb-lsd: <lsd>
x-ig-app-id: 936619743392459
x-asbd-id: 359341
origin: https://www.instagram.com
referer: https://www.instagram.com/<username>/
user-agent: <桌面 Chrome UA>
cookie: <登录 cookie>
```

Form（最小可用，实测不需要 `__dyn/__csr/__hsdp/...` 客户端提示）：

```text
av=<viewer_pk>
__d=www
__user=0
__a=1
__req=1
fb_dtsg=<fb_dtsg>
jazoest=<jazoest>
lsd=<lsd>
fb_api_caller_class=RelayModern
fb_api_req_friendly_name=PolarisProfilePostsTabContentQuery_connection
server_timestamps=true
variables=<JSON>
doc_id=28544666141895803
```

Variables（第一页）：

```json
{
  "after": null,
  "before": null,
  "data": {
    "count": 12,
    "include_reel_media_seen_timestamp": true,
    "include_relationship_info": true,
    "latest_besties_reel_media": true,
    "latest_reel_media": true
  },
  "first": 12,
  "include_multi_captions": true,
  "last": null,
  "username": "nasa",
  "__relay_internal__pv__PolarisMultiCaptionCarouselEnabledrelayprovider": true,
  "__relay_internal__pv__PolarisShortDramaEnabledrelayprovider": true,
  "__relay_internal__pv__PolarisSEOIsCrawlerrelayprovider": false,
  "__relay_internal__pv__PolarisThreadsStyleCommentsEnabledrelayprovider": false,
  "__relay_internal__pv__PolarisReelsRecoDebugOverlayEnabledrelayprovider": false
}
```

翻页：把 `after` 设为上一页 `page_info.end_cursor`，保持其余字段；终止条件 `page_info.has_next_page === false`。每页之间 sleep 1–2s。

## 响应结构

```text
data.xdt_api__v1__feed__user_timeline_graphql_connection.edges[].node
  code            shortcode
  pk              媒体 id
  taken_at        unix 秒
  media_type      1=image 2=video 8=carousel
  product_type    feed / clips
  like_count
  comment_count
  caption.text
  image_versions2.candidates[0].url
  video_versions[0].url
  carousel_media[]
  user.{pk,username,full_name}
page_info.{end_cursor,has_next_page}
```

现有 `apps/server/src/social/instagram-parse.ts` 的 `parseFeedItem()` 字段基本可直接复用（`code/media_type/caption.text/image_versions2/carousel_media`）。

## 实测

- 用项目 `apps/server/data/settings.json` 的 `igCookie`，NASA 主页连续翻 3 页：200，36 条唯一 shortcode，无致命错误。
- 某页出现 1 条非致命 `field_exception`（`profile_pic_url`），不影响帖子节点。
- 本机 Node 直连需要走系统代理：`NODE_USE_ENV_PROXY=1 HTTPS_PROXY=http://127.0.0.1:7897`；浏览器走系统代理，不受影响。
- 旧的 `/api/v1/feed/user/<id>/` 仍可作为 fallback；GraphQL 是当前 Web 端实际路径。

## 2026-10-23 补充：DM / 写操作 doc_id 清单（IG Web）

REA 连接已登录 Chrome 抓 `/api/graphql`，并结合 `static.cdninstagram.com` 线上 bundle 的 `__d("<Op>_instagramRelayOperation",[],...exports="<doc_id>")` 模块整理。doc_id 随 IG 构建变化，仅作当前版本参考；上线应做契约探测与 fallback。

| 操作 | doc_id | 关键 variables |
|---|---|---|
| `PolarisDirectInboxQuery` | 39528552190093449 | `device_id_for_iris_subscription` + 若干 feature flag |
| `IGDThreadDetailQuery` | 39768212509458705 | `thread_fbid`、`min_uq_seq_id`、`IGDInitialMessagePageCount` |
| `PolarisDirectMessageRequestQuery` | 39282737491373568 | 消息请求列表 |
| `IGDInboxTrayQuery` | 29231580869776032 | 托盘未读 |
| `IGDSystemFolderUnreadThreadCountQuery` | 28687343074241941 | 系统文件夹未读 |
| `IGDChatTabsJewelOffMsysQuery` | 30197608969842732 | 聊天 tab jewel |
| `IGDBadgeCountOffMsysQuery` | 28939180542359933 | 全局 badge |
| `PolarisFeedTimelineRootV2Query` | 29836423012624857 | feed 时间线 |
| `IGDirectTextSendMutation` | 26911679871773184 | `ig_thread_igid` 或 `recipient_igids`、`text:{sensitive_string_value}`、`offline_threading_id`、`send_attribution` |
| `IGDirectMediaSendMutation` | 25766288509716264 | `attachment_fbid`、`thread_id`、`offline_threading_id` |
| `IGDirectReactionSendMutation` | 24374451552236906 | `input`（reaction） |
| `IGDirectLinkShareMutation` | 27277149101947973 | `data`/`send_data` |
| `usePolarisFollowUserFollowMutation` | 27767812149509802 | `target_user_id` |
| `usePolarisFollowUserUnfollowMutation` | 25174972798866458 | `target_user_id` |
| `PolarisAPILikePostMutation` | 28051398997871885 | `input`（media） |
| `usePolarisCommentCreateMutation` | 28314137824907940 | `connections` + `data:{comment_text,media_id,replied_to_comment_fbid,tracking_token}` |
| `IGDInboxInfoMuteToggleOffMsysMutation` | 26360506043651125 | `mute_seconds`、`offline_threading_id`、`thread_fbid` |
| `useIGDMarkThreadAsReadMutation` | 27399783383056109 | `data` + `metadata.ig_thread_igid` |
| `PolarisAPIForceStorySeenMutation` | 25133351333028744 | `storyOwnerId` |
| `PolarisAPIReelSeenMutation` | 36133297086317984 | `reelId`、`reelMediaId`、`reelMediaOwnerId`、`reelMediaTakenAt`、`viewSeenAt` |
| `useCreateOpenGroupThreadOffMsysMutation` | 27529949946640544 | `recipient_eimu_fbids`、`offline_threading_id` |
| `usePolarisGetFollowListQuery` | 38705367455744737 | 关注列表 |
| `PolarisFollowListContentWrapperQuery` | 26750505977951904 | 关注列表包装 |

请求形状（实测）：`POST https://www.instagram.com/api/graphql`，`application/x-www-form-urlencoded`，必带 `fb_api_caller_class=RelayModern`、`fb_api_req_friendly_name=<Op>`、`variables=<JSON>`、`doc_id=<id>`；headers 至少 `x-csrftoken`、`x-fb-lsd`、`x-ig-app-id: 936619743392459`、`x-asbd-id: 359341`、`origin`、`referer`、桌面 Chrome UA、登录 cookie。最稳做法是复用页面真实请求的 headers + form body（现有 `interceptUserInfo` 已缓存模板），只替换 `friendly_name` / `doc_id` / `variables`。

其他 REST：`/api/v1/friendships/create|destroy|approve|ignore/{user_id}/`、`/api/v1/news/inbox/`、`/api/v1/news/inbox_seen/`、`/insights/media/{media_id}/...`、`/api/v1/business/account/convert_account/`、`/api/v1/business/account/get_suggested_categories/`。
