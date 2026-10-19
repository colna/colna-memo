---
title: Instagram Web 发帖接口（rupload + configure）实测流程
date: 2026-10-12
tags: instagram, api, create-post, social-proxy, ins-script, sitin-next
---

# Instagram Web 发帖接口（rupload + configure）实测流程

2026-10-12 端到端实测通过：在真实登录会话的浏览器 DevTools 内用 `fetch` 完成，属 IG web 私有接口（非官方 API）。

## 结论速览

「不点击页面、直接调接口发帖」**可行**，两步：

| 步骤 | 请求 | 说明 |
|------|------|------|
| 1. 上传图片 | `POST https://www.instagram.com/rupload_igphoto/fb_uploader_<upload_id>` | body = 图片二进制；`content-type: image/jpeg`；**www 同源即可**（未用到 `i.instagram.com`） |
| 2. 创建帖子 | `POST https://www.instagram.com/api/v1/media/configure/` | form-urlencoded；返回 media 对象（`strong_id__`）即成功 |

`upload_id` = 客户端毫秒时间戳（字符串），两步共用；entity name = `fb_uploader_<upload_id>`。

## 鉴权与动态值

| 值 | 来源 | 备注 |
|----|------|------|
| cookies（sessionid 等） | 浏览器自动携带（same-origin fetch 默认带） | `sessionid` / `www-claim-v2` 都是 **HttpOnly**，页面 JS 读不到、也不需要读 |
| `x-csrftoken` | cookie `csrftoken`（非 HttpOnly，可读） | configure 需要 |
| `x-ig-www-claim` | 理论来源 cookie `www-claim-v2`、或响应头 `x-ig-set-www-claim` | **实测不强制**：为 null 未带，configure 仍 200 |
| `x-ig-app-id` | 固定 `1217981644879628` | |
| `x-instagram-ajax` | 页面 HTML `"rollout_hash":"..."` | 带上，非关键 |
| `fb_dtsg` | 页面 HTML regex `"dtsg":{…"token":"…"}`（或 `require("DTSGInitialData").token`） | |
| `jazoest` | `'2' + sum(dtsg.charCodeAt(i))` | 现算，实测被服务端接受 |

坑：**不能用 `document.cookie` 里的 `sessionid` 判断登录态**（HttpOnly 恒读不到，会误判未登录）。

## 上传请求（步骤 1）

headers：

- `content-type: image/jpeg`、`offset: 0`
- `x-entity-length: <字节数>`、`x-entity-name: fb_uploader_<upload_id>`、`x-entity-type: image/jpeg`
- `x-ig-app-id`、`x-instagram-ajax`
- `x-instagram-rupload-params: {"media_type":1,"upload_id":"<upload_id>","upload_media_height":1080,"upload_media_width":1080}`

成功响应：`{"upload_id":"<upload_id>","status":"ok"}`

## configure 请求（步骤 2）

form（照抄真实抓包）：

```
caption=<文案>
clips_share_preview_to_feed=1
disable_comments=0
like_and_view_counts_disabled=0
media_share_flow=creation_flow
source_type=library
upload_id=<upload_id>
video_subtitles_enabled=0
custom_accessibility_caption=accessibility
jazoest=<'2'+charCodeSum(dtsg)>
fb_dtsg=<页面 dtsg>
```

headers：`content-type: application/x-www-form-urlencoded`、`x-csrftoken`、`x-ig-app-id`、`x-requested-with: XMLHttpRequest`、`x-asbd-id: 359341`、`x-instagram-ajax`（可选）、`x-ig-www-claim`（可选）。

成功响应：`{"media":{"strong_id__":"<media_id>_<uid>",...}}`（200）。

## 实测记录（2026-10-12）

- 环境：桌面 Chrome，DevTools Console；图片为 canvas 现场生成的 1080x1080 JPEG（40.6KB）。
- 上传：`POST /rupload_igphoto/fb_uploader_1791775720937` → 200 `{"upload_id":"1791775720937","status":"ok"}`。
- 创建：configure → 200，media `4006027583603056138_43150596056`（测试帖，caption `sp probe test … (ignore)`）。
- 未验证：story；桌面 web 官方照片发帖 UI 是否同参数（本次按手机 web 抓包复刻）。多图 / 图+视频 carousel 已于 2026-10-13 实测抓包（见下文「carousel」节）。

## 视频（Reels）发帖流程（2026-10-12 实测，桌面 Chrome）

视频走 Reels 管线，比照片多两步且需等转码，**4 步 + 转码轮询**：

| 步骤 | 请求 | 说明 |
|------|------|------|
| 1. 初始化 | `GET https://i.instagram.com/rupload_igvideo/fb_uploader_<upload_id>` | 空 body；返回 `{"dc":"…","offset":0}`；**跳过此步后续 POST 会 404 `Unknown entity`** |
| 2. 上传视频 | `POST https://i.instagram.com/rupload_igvideo/fb_uploader_<upload_id>` | body = 视频二进制；成功返回 `{"media_id":"…","status":"ok"}`（注意不是 upload_id） |
| 3. 上传封面 | `POST https://i.instagram.com/rupload_igphoto/fb_uploader_<upload_id>` | 从视频抽帧的 JPEG（`x-entity-type: image/jpeg`）；成功 `{"upload_id":"…","status":"ok"}` |
| 4. 创建 | `POST /api/v1/media/configure_to_clips/` | form 同照片版 + `archive_only=false`、`disable_oa_reuse=false`（无 custom_accessibility_caption）；**202 `Transcode not finished yet` 需轮询重试**，200 即成功 |

关键点：

- 视频 rupload-params（步骤 2）：`{"client-passthrough":"1","is_clips_video":"1","for_album":false,"is_sidecar":"0","media_type":2,"upload_id":"…","upload_media_duration_ms":1579,"upload_media_height":1920,"upload_media_width":1080,"video_edit_params":{crop_height,crop_width,crop_x1,crop_y1,mute,trim_end,trim_start},"video_format":"","video_transform":null}`
- 封面 rupload-params（步骤 3）精简：`{"media_type":2,"upload_id":"…","upload_media_height":…,"upload_media_width":…}`（media_type 仍 2）
- 步骤 2 请求**不带** `x-entity-type`（照片带）；步骤 2/3 走 `i.instagram.com`（CORS 允许 `www.instagram.com` 来源 + credentials，实测可用）
- `X-Web-Session-ID`（`xxx:xxx:xxx` 格式）可从页面 sessionStorage/localStorage 取到；不带也能过
- 转码轮询实测：第 1 次 202 → 2.5s 后第 2 次 200（`subtype_name_for_REST__: "XDTClipsMedia"`）
- 封面帧获取：页面内 `<video>` + canvas 抽帧 → JPEG blob（无需用户上传封面）
- configure 的错误响应（如 408）可能带 `Access-Control-Allow-Origin: *`，在 `credentials: include` 下被浏览器拒 → 表现为 CORS 报错，勿误判为上传失败

## 多图 / 图+视频 carousel（2026-10-13 实测抓包）

「图 + 视频」混发手动抓包校准（对应脚本 `createPost` 的 carousel 分支）：

- 每个 child 独立上传、独立 `upload_id`：
  - 图片 child 走 `i.instagram.com/rupload_igphoto`，rupload-params 干净版 `{"media_type":1,...}`
  - 视频 child 的封面帧走 `rupload_igphoto`，`{"media_type":2,...}`
- 创建走 `POST /api/v1/media/configure_sidecar/`，与 configure 差异：
  - **JSON body**（非 form-urlencoded），带 `children_metadata: [{upload_id}, …]`
  - **`client_sidecar_id` 独立生成**（与各 child 的 upload_id 不同）
  - **`x-ig-app-id: 936619743392459`**（web 端 id；单图/视频用的 `1217981644879628` 不适用）
  - 202 轮询同 `configure_to_clips`
- 遗留不确定：carousel 内视频 child 的 `rupload_igvideo` params（clips/sidecar 标志）无抓包，按「干净版」推断；`x-ig-www-claim` 省略（单图经验不强制）。
- 脚本入口：`createPost({files, caption, postKey})` —— 1 视频 → Reels 四步；1 图 → 照片 configure；2+ → 本 carousel 流程。`videoUrl` / `imageBase64` 入参已移除。

## CSP 限制（重要）

IG 页面 `connect-src` 为白名单（instagram / facebook / fbcdn / cdninstagram 等系），**页面内 fetch 任何第三方 URL 都会被 CSP 直接拦截**（实测 `file.archat.us` 被拒）。因此：

- 素材输入的生产路径：URL 经 SP `/script/media-chunk` 分片接口 + 容器原生 `requestHttp` 拉取（绕开页面 fetch）；脚本入参统一为 `files` 数组
- 调试时可用「拖拽文件」替代（本地文件不走网络，不受 CSP 限制）

## 注意

- 照片上传 www 同源即可；视频流程走 `i.instagram.com`（跨子域但 CORS 已允许，实测通过）。
- 私有接口 + 风控：发内容是 IG abuse 高发区，自动化发帖比评论更敏感，频率与行为需克制。
- 与 DOM 点击方案对比：接口方案不受页面改版影响、无「本地文件选择」限制（二进制可直接 fetch 上传）；DOM 方案更接近真人路径但脆弱。

## Story（configure_to_story）测试脚本（2026-10-19，待实测）

脚本：`packages/app-ins-scripts/scripts/test-ins-story.js`（DevTools Console 直接粘贴，图片/视频均支持）。

- 底层按 mobile private API 复刻：`POST https://i.instagram.com/api/v1/media/configure_to_story/`，body 为 `ig_sig_key_version=4&signed_body=<hmac_sha256>.<json>`（签名 key 见脚本；公开 IG 客户端 key）
- payload 关键字段：`source_type=4`、`upload_id`、`story_media_creation_date`、`client_shared_at`、`client_timestamp`、`configure_mode=1`、`edits/extra`、`jazoest`、`_csrftoken/_uid/_uuid/device`
- 视频：`rupload_igvideo` → 封面上传（`media_type=2`）→ configure 封面（不带 `?video=1`）→ `configure_to_story/?video=1`；202 / `Transcode not finished` 需轮询
- 参考实现：`jackwener/opencli` 的 `clis/instagram/_shared/private-publish.js`（同签名/同 payload）
- 状态：语法检查通过（`node --check`），尚未在真实账号上端到端实测；桌面 web 无 Story 入口，只能走接口或手机 App。
- 坑：`rupload_igphoto` / `rupload_igvideo` 的 CORS 白名单**不含** `X-CSRFToken`、`X-Requested-With`；上传请求带上会在预检直接失败（`x-requested-with is not allowed`）。上传头只带 `X-ASBD-ID` / `X-IG-App-ID` / `X-Instagram-AJAX` / `X-Web-Session-ID` + `X-Entity-*` / `Offset` / `X-Instagram-Rupload-Params`；CSRF 头只给 configure。
