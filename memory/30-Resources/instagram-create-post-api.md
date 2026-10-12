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
- 未验证：视频 / 多图 carousel / story；`i.instagram.com` 跨域上传（未用到）；桌面 web 官方发帖 UI 是否同参数（本次按手机 web 抓包复刻）。

## 注意

- 跨子域上传（`i.instagram.com`）会触发 CORS preflight，优先用 www 同源路径。
- 私有接口 + 风控：发内容是 IG abuse 高发区，自动化发帖比评论更敏感，频率与行为需克制。
- 与 DOM 点击方案对比：接口方案不受页面改版影响、无「本地文件选择」限制（二进制可直接 fetch 上传）；DOM 方案更接近真人路径但脆弱。
