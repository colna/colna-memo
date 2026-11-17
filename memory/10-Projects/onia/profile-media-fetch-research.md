---
title: "Onia 回关后抓取对方帖子与 Story 调研"
date: 2026-11-17
tags: project, onia, instagram, social-proxy, posts, story, research
---

# Onia 回关后抓取对方帖子与 Story 调研

日期：2026-11-17。对照本机 `/Users/colna/WORK/onia`、`sitin-next/packages/app-ins-scripts`、`app-social-proxy-server` 与 colna-memo 既有排错笔记。

## 结论

- SP 没有现成的「IG 对方 Story 内容」脚本；IG 帖子只有 `getProfilePostImage`（主页九宫格封面，靠 DOM 滚动），信息量低、速度慢。
- 能做到「帖子正文/指标」的 GraphQL 查询已有实测记录：`PolarisProfilePostsTabContentQuery_connection`，doc_id `28544666141895803`；`getProfileReels` 曾实现但已删除。
- 建议 Onia 新写 `get_profile_media`：回关 `follow_user` 成功后作为同 Signal 第 2 步执行；API-first（GraphQL posts + `feed/reels_media` stories），DOM 只作 fallback。独立动作失败不影响回关成功。

## SP 脚本盘点

| 脚本 | 能拿到 | 是否适合回关后抓取 | 限制 |
|---|---|---|---|
| `getProfileInfo.js` | profile：pk、username、full_name、bio、bio_links、follower/following/media_count、is_private、`latest_reel_media`、`has_story_archive` | 作为「是否有活跃 Story」的前置判断 | 不含帖子和 Story 内容；`latest_reel_media` 是时间戳，不是媒体 |
| `getProfilePostImage.js` | 主页九宫格：`href`、`shortcode`、`type`、`coverUrl`、`alt`，默认最多 30 条 | 是 SP 目前唯一稳定的 IG 帖子脚本 | 只有封面；不含 caption/点赞/评论/视频地址；需导航到主页并滚动 |
| `getMediaInsights.js` | 单条 media 的 insights | 不适合 | 只对自有媒体有意义，取不到对方帖子 |
| `getProfileReels.js` | `PolarisProfileReelsTabContentQuery`：play_count、like_count、comment_count、code、pk、product_type | 可作 Reels 补充 | 文件已删除，只剩 KB 里的接口规格 |
| `getStoryViews.js` | 自己 Story 的浏览量和观众 | 不适合 | 仅作者本人；它证明 `feed/reels_media` 能返回 `reels[userId].items` |
| `snapchat/getFriendStory.js` | 好友 Story：逐页 image/video/base64 | 仅 Snapchat | 靠 DOM 播放器，不能直接用于 IG |
| `snapchat/getStory.js` | 自己 Story | 不适合 | Snapchat 专用 |
| `createStory.js` | 发自己的 Story | 不适合 | 发布，不是抓取 |

SP v3 的映射也印证了缺口：`fetch-profile-images.def.ts` 对 IG 只调 `getProfilePostImage`，对 Snapchat 才调 `EXTRACT_STORY`；`EXTRACT_STORY` 映射的是 `getStory`（自己的 Story），不是对方的。

## 可用接口

### 对方帖子（已验证）

- endpoint：`POST /api/graphql`
- operation：`PolarisProfilePostsTabContentQuery_connection`
- doc_id：`28544666141895803`（IG 构建会轮换，需配置 + 探测）
- variables 关键字段：`username`、`first`、`after`、`data.{count,include_relationship_info,latest_reel_media}`、`__relay_internal__pv__*`
- 响应：`data.xdt_api__v1__feed__user_timeline_graphql_connection.edges[].node`
  - `code`（shortcode）、`pk`、`taken_at`、`media_type`（1/2/8）、`product_type`、`like_count`、`comment_count`、`caption.text`、`image_versions2.candidates[0].url`、`video_versions[0].url`、`carousel_media[]`
- 翻页：`after = page_info.end_cursor`，`page_info.has_next_page === false` 停止；建议每页 sleep 1–2s。

### Reels（已验证，可选）

- operation：`PolarisProfileReelsTabContentQuery`，doc_id `39368442709468666`
- 响应：`data.fetch__XDTUserDict.clips_connection.edges[].node.media`，`play_count` 是前台播放量，`view_count` 为 null。
- 若只抓主页帖子，可先不接入；需要 Reels 播放量时再加。

### 对方 Story（接口路径已存在，内容需真机复核）

- `GET /api/v1/feed/reels_media/?reel_ids=<target_pk>`（WebView 同源）
- 响应：`data.reels[<target_pk>].items[]`，每项含 `pk`/`id`、`media_type`、`taken_at`、`expiring_at`、`image_versions2`、`video_versions`、`caption` 等。
- `getProfileInfo` 的 `latest_reel_media` 可先判断有没有活跃 Story，避免无意义请求。
- 既有笔记只验证过它作为 `list_reel_media_viewer` 的回退能拿到 `items[].viewer_count`；「拉整组当前 Story」需要真机复核，且私有账号需要对方已接受关注。
- 备选：打开 `/stories/<username>/` 页面预加载 `PolarisStoriesV3ReelPageStandaloneQuery`（doc_id `29170422829314651`）；但需要先有 story media id，更适合作为单条详情，不作为列表入口。

## Onia 落地方案

推荐：**独立动作，不把抓取塞进 `follow_user`**。

1. 协议：`crates/onia-protocol` 新增 cap `profile.media.v1`、action `get_profile_media`、todo `GET_PROFILE_MEDIA`；params：`target_handle`（必填）、`max_posts`（默认 12）、`include_stories`（默认 true）、`pages`（默认 1）。
2. 策略：`onia-policy/src/lib.rs` 的 `new_follower_pack` 在原有 `follow_user` 后追加第二个 `TodoPlan`（`step_index: 2`、`priority: 29`、`max_attempts: 1`、`required_capability: profile.media.v1`）。Scheduler 现有「前一步未成功不晋升后一步」的门禁可直接复用；抓取失败不会影响回关成功。
3. 脚本：`packages/scripts/src/actions/profile_media.ts` + `instagram/profile_media.ts`；`execute.ts` 注册 `get_profile_media`；`max_posts` 建议 12（最多 30），Story 只取当前 24h。
4. 返回结构：
   `{ target_handle, target_id, private, posts: [{shortcode, pk, media_type, product_type, taken_at, like_count, comment_count, caption, cover_url, video_url, carousel_count, post_url}], stories: [{media_pk, media_type, taken_at, expiring_at, cover_url, video_url}], story_available, partial, fetched_at }`
5. 存储：MVP 直接落 `behaviors.result`（已是 JSONB）；若要给 Agent/后台查询，再加 `profile_media_snapshots` 表，按 `(user_id, platform, account_id, owner_handle, kind, media_id)` upsert，媒体文件不进 PostgreSQL，只存 URL 和元数据。
6. 客户端：`apps/client/lib/config/defaults.dart` 与 `device_runtime.dart` 增加 `profile.media.v1`；`apps/dev-device` 同步；否则调度会按「设备缺能力」取消。
7. 失败语义：私有账号/关注请求未通过 → `private: true`、空列表、成功返回；Story 为空 → 空列表；GraphQL doc_id 失效 → 降级到 `getProfilePostImage` DOM 方案或返回 `partial: true`。
8. 频控：每个 follower 只抓一次，1–2 页；Story 仅在 `latest_reel_media` 近期存在时抓；避免回关高峰连续打 GraphQL。

## 待确认

1. 平台范围：只 IG，还是 Snapchat 也要？
2. 「帖子」范围：最近 12 条够不够？是否必须包含 Reels 播放量？
3. Story 范围：只要当前 24h，还是 highlights/archive 也要？
4. 数据用途：只存 `behaviors.result` 供追踪，还是落表给 Agent/后台？
5. 私有账号回关后对方尚未通过关注请求时拿不到内容，是否接受空结果？

## 参考

- `30-Resources/troubleshooting/instagram-web-posts-graphql.md`
- `30-Resources/troubleshooting/instagram-story-viewers-api.md`
- `sitin-next/packages/app-ins-scripts/src/instagram/methods/getProfilePostImage.js`
- `sitin-next/packages/app-ins-scripts/src/instagram/methods/getProfileInfo.js`
- `sitin-next/packages/app-social-proxy-server/src/v3/todo/definitions/fetch-profile-images.def.ts`
- `sitin-next/packages/app-social-proxy-server/src/v3/behavior/apk-script-map.ts`
