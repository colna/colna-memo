---
title: AIGC-Studio 生图接口排错
date: 2026-09-09
tags: troubleshooting, aigc-studio, image-generation, cloudflare, gpt-image
---

# AIGC-Studio 生图接口

## 接口口径(2026-09-09 实测跑通)

站点 `https://aigc-studio.sitinai.com`(Cloudflare 前置 → 1Panel openresty → `127.0.0.1:3000`)。

```
POST {BASE}/api/generations   {modelId, input:{prompt, aspectRatio, images?}}  → {id}
GET  {BASE}/api/generations/{id}       → {status, outputUrl, thumbnailUrl, error}
POST {BASE}/api/uploads       multipart 字段名 file → {url}
Cookie: aigc_session=<JWT>
```

- status:`pending → processing → done / failed`
- 模型 `gpt-image-2` / `gpt-image-2-all`;画幅只认 `1:1`(1024²)/`3:2`(1536×1024)/`2:3`(1024×1536)/`auto`
- 参考图**最多 1 张**、**只接 URL 不接 base64**,本地图先 `POST /api/uploads` 换 OSS 公共 URL
- cookie 是 jose HS256 JWT,**7 天过期**(`web/lib/session-token.ts` 的 `SESSION_COOKIE`)
- 实测出图 **~125s**,1254×1254 PNG 2.6MB,落 `n8n-video-resources.oss-us-west-1.aliyuncs.com`

**`/api/enqueue` 不是生图入口** —— 它只往 `PING_QUEUE` 发一条测试消息,看名字最容易用错。

## `403 error code 1010`

- **现象**:curl 打 `/api/health` 好好的,换 Python `urllib` 请求同一站点直接 403,body 是 `error code: 1010`。
- **根因**:1010 是 **Cloudflare 按 UA 指纹拒绝**,默认的 `Python-urllib/3.x` 命中黑名单。与 cookie、路由、证书都无关。
- **修法**:请求头固定带浏览器 UA。判据是「curl 通、脚本不通」——先怀疑 UA,别去查鉴权。

## 怎么找站点域名(找了很久的弯路)

仓库里**没有**公网域名:`deploy/openresty/aigc-studio.conf` 的 `server_name` 是 `your-domain.example.com`,
`deploy/README.md` 写「主域名:对外域名」,`.A_memory` 只说「域名已指向服务器」。
`deploy/push.sh` 里的 `HOST=root@10.1.25.242` 是内网机,**ping 通但 3000 端口只绑 `127.0.0.1`**,
直连 80 会落 openresty 默认站点报 404、443 因证书按域名签发而握手失败。
**结论:域名只能问人,别在仓库里翻。**

## OUTPUTS_DIR 未必存在

CLAUDE.md 说输出文件放 `$OUTPUTS_DIR`,但**实测该变量没导出到 shell**(读出来是空)。
脚本里拿它当默认值会静默落到当前目录。要发回飞书就传绝对路径,或落盘后 `cp` 到
系统提示给的 `.../metabot-outputs-max/<chat_id>`。
