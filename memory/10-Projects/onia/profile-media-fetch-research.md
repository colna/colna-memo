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

## 已确认范围（2026-11-17）

- 只做 Instagram。
- 「帖子」= 全量，不设 12 条上限；同时拉 Reels 和 `play_count`。
- Story = 当前 24h + highlights；archive 也尝试，但 IG 只允许账号本人读取自己的 archive，别人的 archive 不可见。
- 数据落表，供 Agent / 后台查询，不只放 `behaviors.result`。
- 私有账号/关注请求未通过时接受空结果，不报失败。
- 动作名继续用 `get_profile`，不新增 `get_profile_media`；`get_profile` 现在返回完整资料 + 媒体。

## 最终实现

### 协议 / 策略

- `crates/onia-protocol`：新增 cap `profile.media.v1`；`get_profile` 的 required caps 变为 `profile.lookup.v1 + profile.media.v1`；params 增加 `include_posts`、`include_reels`、`include_stories`、`include_highlights`、`include_archive`、`max_posts`、`max_pages`、`page_size`、`time_budget_ms`、`page_delay_ms`、`posts_after`、`reels_after`、`target_id`、`skip_meta`、`auto_continue`。
- 新增 signal：`PROFILE_POSTS_PAGE`、`PROFILE_STORIES_PAGE`、`PROFILE_HIGHLIGHTS_PAGE`、`PROFILE_FETCH_NEEDED`。
- `onia-policy`：`new_follower_pack` 在 `follow_user` 后追加第 2 步 `get_profile`（`step_index: 2`，`priority: 29`），复用 Scheduler 前驱门禁；新增 `profile_media_pack` 处理系统续拉信号。

### 脚本

- `packages/scripts/src/instagram/profile_media.ts`：
  - 帖子：`PolarisProfilePostsTabContentQuery_connection`（doc `28544666141895803`），按 `page_info.end_cursor` 翻页。
  - Reels：`PolarisProfileReelsTabContentQuery`（doc `39368442709468666`），带 `play_count`。
  - 当前 Story：`GET /api/v1/feed/reels_media/?reel_ids=<pk>`，失败时回退 `PolarisInstapi.apiGet`。
  - Highlights：`/api/v1/highlights/<pk>/highlights_tray/` + 每个 highlight 的 `feed/reels_media?reel_ids=highlight:<id>`。
  - Archive：仅当目标 pk 等于当前登录 `ds_user_id` 时调用 `/api/v1/archive/reel/day_shells/`；否则明确返回不可见。
  - 帖子/Reels 每页通过 `reportSignal` 上报 `PROFILE_POSTS_PAGE`，避免把全量数据塞进 `behaviors.result` 和 SharedPreferences。
  - 默认 `time_budget_ms=30s`（低于 Flutter JS bridge 45s，超时由服务端续拉）；`max_posts=0`、`max_pages=0` 表示不限。
- `packages/scripts/src/actions/instagram_actions.ts`：
  - 原轻量查询改名为内部 `lookupProfile`，`follow_user` 继续用它。
  - 导出 `getProfile` 变成完整动作：先 `lookupProfile`（续拉时 `skip_meta` 跳过），再抓 stories/highlights/archive/posts/reels。
  - 返回资料字段（头像、昵称、bio、links、粉丝/关注/media_count、认证/商业/地区等）+ `stories`、`highlights`、`archive`、`pagination`、`stats`、`partial`、`errors`。

### 服务端落表

迁移 `crates/onia-server/migrations/0009_profile_media.sql`：

- `profile_snapshots`：每次完整抓取的资料快照（profile JSONB、stats、errors、partial）。
- `profile_posts`：帖子 + Reels，按 `(user_id, platform, account_id, owner_handle, shortcode)` upsert；含 caption、点赞/评论/播放量、封面/视频、carousel、raw。
- `profile_stories`：Story / highlight item / archive item，按 `source + highlight_id + media_pk` upsert。
- `profile_highlights`：highlight 元数据 + item_count。

`crates/onia-server/src/profile_media.rs`：

- `apply_signal` 处理三种 page signal，批量 upsert。
- `apply_get_profile_result` 写 snapshot（`skip_meta` 续拉不写），并在 `has_more_posts` / `has_more_reels` 时 ingest `PROFILE_FETCH_NEEDED`，由 policy 生成下一次 `get_profile`。
- `pipeline::ingest` 后调用 `apply_signal`；`apply_behavior_result` 成功后调用 `apply_get_profile_result`。

### 客户端

- `profile.media.v1` 加入 `apps/client/lib/config/defaults.dart`、`device_runtime.dart`、`apps/dev-device`。
- `get_profile` 的 JS bridge 超时从 45s 放宽到 75s；旧客户端未更新时，第二步会因缺 cap 被取消，但不影响回关成功。

## IG 限制与风险

- **别人的 archive 拿不到**：IG 只允许账号本人看自己的 archive；代码已做 `ds_user_id` 校验，避免把 viewer 的 archive 写到目标 handle 下。
- **全量帖子会分批**：单次 `get_profile` 受 30s 预算限制；服务端用 `PROFILE_FETCH_NEEDED` 自动续拉，直到 `has_next_page=false`。高粉账号可能跨多个 behavior，仍受 IG 限流影响。
- **doc_id 轮换**：帖子/Reels 的 doc_id 随 IG 构建变化；失败会返回 `partial/errors`，不会伪造数据。
- **私有账号**：对方未接受关注请求时，帖子和 Story 返回空；这是权限边界，不是脚本错误。
- **全量 follower/following 列表未拉**：只保留关注状态和 counts；拉完整列表成本高且隐私敏感，需要时再单独立项。

## 待确认（后续）

1. 后台是否需要 `/v1/admin/profile-media` 查询接口与页面；当前只落表，Agent 可直接读 PostgreSQL。
2. 自己的 archive 是否需要手动入口（`include_archive: true`）；目前只在目标等于当前账号时生效。
3. 是否需要 tagged posts、followers/following 全量列表等扩展。
## 参考

- `30-Resources/troubleshooting/instagram-web-posts-graphql.md`
- `30-Resources/troubleshooting/instagram-story-viewers-api.md`
- `sitin-next/packages/app-ins-scripts/src/instagram/methods/getProfilePostImage.js`
- `sitin-next/packages/app-ins-scripts/src/instagram/methods/getProfileInfo.js`
- `sitin-next/packages/app-social-proxy-server/src/v3/todo/definitions/fetch-profile-images.def.ts`
- `sitin-next/packages/app-social-proxy-server/src/v3/behavior/apk-script-map.ts`
