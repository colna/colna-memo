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
- 参考图口径**看部署版本**:
  - 线上现行(截至 2026-09-11 未更新):**最多 1 张**、**只接 URL 不接 base64**,本地图先 `POST /api/uploads` 换 OSS 公共 URL
  - 分支 `feature/aigc-dev` 已改成 **最多 4 张 + 接受 data URI**(`refImages: {max:4, inline:true}`),
    多张时 multipart 字段名是重复的 `image[]`。**callapi 是否真透传多图尚未实测**,上线前先打一发双图请求验。
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

## 要「照 B 图的样子改 A 图」但只能传 1 张参考图

> 这条是线上单图版本下的绕法;`feature/aigc-dev` 放开到 4 张后,双图可以直接喂进去,不必再靠文字描述。

- **约束**:driver 的 `refImages: { max: 1 }`,`/images/edits` 只吃一张图。想「把 A 图里的角色换成 B 图里的角色」时,两张图喂不进去。
- **做法**:**传要改的那张(A)当参考图,把 B 的形态用文字写死在 prompt 里**。
  文字描述若已有权威版本(如 `Luka-Swipe-Deck-Prompt-EN.md` 的 MASCOT 段落),直接整段抄进去,别自己重写 —— 那段本来就是角色设定的真源。
- **prompt 结构**(实测有效):
  1. `Edit the attached image. Change ONE thing only: REPLACE X ...`
  2. **KEEP EXACTLY AS IS** 逐项枚举要保留的元素(背景/文案原文/装饰/构图/姿势),越具体越不会被重画
  3. 角色设定整段
  4. **WHAT MUST VISIBLY CHANGE**:逐条写「现在这只哪里错了、应该是什么样」—— 只给正面描述模型会偷懒不改
- **实测残留**:细小特征(如「胡须几乎看不见」)最难压住,模型倾向画显眼的长胡须。这类特征要在 3 和 4 里**重复两次**才有机会生效。

## 要透明底:站点这条路做不到,得「纯白底 + 本地抠」

- **根因**:`drivers/gpt-image.ts` 的 `buildSubmit` 只发
  `{model, prompt, n, size, response_format}`,**没有 OpenAI 的
  `background: "transparent"`**。所以走 AIGC-Studio 永远拿不到真透明 PNG。
  (要真透明得改仓库那一处 + 确认 callapi.top 跟进了该参数。)
- **折中办法**:出纯白底再抠,两步都有讲究。
  1. prompt 里必须同时写死这几条,少一条就白干:
     ```
     PURE FLAT WHITE #FFFFFF background and nothing else.
     NO glow, NO halo, NO vignette, NO gradient, NO coloured rim light.
     NO drop shadow, NO contact shadow, NO ground plane, NO reflection.
     ```
     **模型默认会给暖光晕 + 接地阴影**,抠完就是一圈脏边。
  2. `.claude/skills/image-gen/scripts/cutout_bg.py`(工作区 skill)。
- **抠图关键:不能「把白像素全变透明」**——角色身上的白毛会被打穿。
  正确做法是**从画布四边 flood fill**,只有与边缘连通的背景才去掉;角色内部的白色
  被前景包围,碰不到。边界按「离背景色的距离」给部分 alpha,毛发边缘才是软过渡。
- **验收**:把结果叠到深青 / 深洋红这类深色底上看白边,别只看棋盘格预览。
- 依赖 numpy + Pillow,**系统 `/usr/bin/python3`(3.9) 两个都有**;Homebrew python3 没有。

## 透明底的正解:供应方支持 `background: "transparent"`(2026-09-10 实测)

前面那套「纯白底 + `cutout_bg.py` 抠图」是**因为 AIGC-Studio 的 driver 没传参数**才需要的折中。
直连 OpenAI 兼容中转时,**直接要真 alpha**:

```jsonc
{
  "model": "gpt-image-2",
  "prompt": "...",
  "size": "1024x1024",
  "response_format": "b64_json",
  "background": "transparent",   // ← 关键
  "output_format": "png",
  "quality": "high"
}
```

- 实测 `api.muskapi.cc/v1` 支持:返回 `background: "transparent"`,PNG 为 **RGBA**,
  alpha 范围 `(0, 254)` —— 真透明,不用抠。
- **prompt 里仍要写死** `NO drop shadow / contact shadow / ground plane / reflection`,
  否则模型会把阴影画进不透明像素里,alpha 通道救不了。
- 响应还回 `usage`(input/output tokens、image_tokens),可用来算成本。
- 耗时:1024×1024 约 **41~55s**(quality medium/high 差别不大)。
- `quality` 传 `high` 时响应里回显的仍是 `medium` —— 该中转可能没透传,别当成功验证。

**判断某个中转支不支持**:发一次带 `background: "transparent"` 的请求,
看响应 JSON 的 `background` 字段和产物的 PIL `mode`(要 `RGBA`)+ alpha extrema(最小值要为 0)。
