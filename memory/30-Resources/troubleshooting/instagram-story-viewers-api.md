---
title: Instagram Story 浏览量接口（list_reel_media_viewer）
date: 2026-10-29
tags: [troubleshooting, instagram, story, api, web]
---

# Instagram Story 浏览量接口

2026-10-29 用真实登录 session 实测：只要拿到 story 的 media pk，就能查 **浏览量 + 观众名单**。仅 story 作者本人有权限。

## 结论速览

| 用途 | 接口 |
|------|------|
| 主接口（mobile host，GET） | `GET https://i.instagram.com/api/v1/media/<media_id>/list_reel_media_viewer/` |
| Web 同源回退（POST） | `POST https://www.instagram.com/api/v1/media/<media_id>/list_reel_media_viewer/` |
| Story 页预加载字段 | `PolarisStoriesV3ReelPageStandaloneQuery` 的 `viewer_count` / `viewers` |
| 最后回退 | `feed/reels_media` 的 `items[].viewer_count` |

## 主接口（i.instagram.com GET）

```text
GET https://i.instagram.com/api/v1/media/4018482142593589163/list_reel_media_viewer/
```

Headers（实测最少）：

```text
User-Agent: <桌面 Chrome UA>
x-ig-app-id: 936619743392459
x-csrftoken: <cookie csrftoken>
Accept: */*
Cookie: sessionid=...; csrftoken=...; ds_user_id=...; ig_did=...; mid=...
```

`x-ig-www-claim` 实测没带也成功；带上也不影响。

响应（200）：

```json
{
  "next_max_id": null,
  "user_count": 3,
  "total_viewer_count": 3,
  "users": [
    { "pk": "3676482788", "username": "newlundavid", "full_name": "David Newlun" },
    { "pk": "42502647106", "username": "renetrickett", "full_name": "Rene Trickett" },
    { "pk": "61256305066", "username": "highlandsmc_mojoshuttle", "full_name": "Matty C" }
  ]
}
```

- `total_viewer_count` 是总浏览量；
- `user_count` 是当前页/总人数（小列表时与总浏览量一致）；
- `users` 是观众列表，`next_max_id` 非空时用 `max_id` 继续翻页。

## Web 同源回退（www POST）

`GET https://www.instagram.com/api/v1/media/<media_id>/list_reel_media_viewer/` 会返回 404 HTML；**必须 POST**：

```text
POST https://www.instagram.com/api/v1/media/4018482142593589163/list_reel_media_viewer/
Content-Type: application/x-www-form-urlencoded

include_blacklist_sample=true
```

返回结构与 i.instagram.com 一致（`total_viewer_count` / `user_count` / `users` / `next_max_id`）。

在 WebView 里优先用页面内 `PolarisInstapi.apiPost`：

```js
require('PolarisInstapi').apiPost(
  '/api/v1/media/{media_id}/list_reel_media_viewer/',
  { body: { include_blacklist_sample: true }, path: { media_id: mediaId } }
)
```

这是 IG Web 自己的 `PolarisStoryAPIActions.fetchReelsViewers` 走的同一条路径。

## Story 页预加载字段

打开 `/stories/<username>/<media_id>/` 时，页面会预加载 `PolarisStoriesV3ReelPageStandaloneQuery`（doc_id `29170422829314651`）：

```json
{
  "variables": {
    "media_id": "4018522906699191852",
    "reel_ids_arr": ["2006516245"],
    "__relay_internal__pv__PolarisCommunityNoteStoriesLabelEnabledrelayprovider": true
  }
}
```

媒体 dict 里的关键字段：

```json
{
  "id": "4018522906699191852_2006516245",
  "pk": "4018522906699191852",
  "viewer_count": 0,
  "viewers": [],
  "taken_at": 1793265282,
  "media_type": 1
}
```

- 本人 story：`viewer_count` 是数字（0/3/…），`viewers` 是观众；
- 他人 story：`viewer_count: null`、`viewers: []` —— 看不到浏览量，这是权限边界，不是 0。

## 实测记录（2026-10-29）

| story media id | 账号 | 结果 |
|---|---|---|
| `4018482142593589163` | `68813149232` 的 session | `total_viewer_count=3`，观众 `newlundavid` / `renetrickett` / `highlandsmc_mojoshuttle` |
| `4018522906699191852` | 本人 `tawfeeqbarakatzwj` | story 页预加载 `viewer_count=0`、`viewers=[]` |
| `4017874403835908005` | 他人 `mariacabreradpaulino` | `viewer_count=null`、`viewers=[]`，只能看内容 |

## 相关脚本

`packages/app-ins-scripts/src/instagram/methods/getStoryViews.js`：

- `SocialProxy.getStoryViews({ mediaId })`
- pipeline：`获取 Story 浏览量 → 解析浏览量`
- 优先 `i.instagram.com` GET；失败回退 www `PolarisInstapi.apiPost`；再失败回退 `feed/reels_media`
- 只依赖 `mediaId`；`userId` 仅最后的 `feed/reels_media` 回退需要
- 返回 `{ mediaId, viewerCount, userCount, viewers, nextMaxId, hasMore, source }`

## 坑

- `feed/reels_media` 在服务端/curl 直接调很容易 `1357004`（`Please try closing and re-opening your browser window`）；这是缺少浏览器指纹头，不是参数错。WebView 内用 `PolarisInstapi` / `PolarisStoryAPIActions` 最稳。
- `www.instagram.com/.../list_reel_media_viewer/` 的 GET 是 404，必须 POST；`i.instagram.com` 的 GET 可以。
- `sessionid` 在 www 是 host-only cookie 时，`i.instagram.com` 可能取不到；所以保留 www POST 回退。
- `mediaId` 可能是 `4018522906699191852_2006516245` 或短码 URL，先取纯数字 pk。
- 只有作者本人有权限；查别人会 403 / `login_required`，或在 story 页返回 `viewer_count: null`。
- 归档 story 走 `GET /api/v1/archive/reel/day_shells/`，items 里同样有 `viewer_count`。

## 参考

- [Instagram Web 主页帖子 GraphQL 逆向](instagram-web-posts-graphql.md)
- [Instagram Web 发帖接口](instagram-create-post-api.md)
