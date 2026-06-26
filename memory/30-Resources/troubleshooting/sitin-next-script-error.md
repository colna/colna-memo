---
title: sitin-next sp-follow 错误日志诊断笔记
date: 2026-06-26
tags: troubleshooting, sitin-next, instagram, android, webview
---

# sitin-next sp-follow 错误日志诊断笔记

## 1. `Cannot read properties of undefined (reading 'execute')`

**坑**:server 端 `social_proxy_script_error_logs` 表里大量这条裸字符串错误,code 列空、无 dom_snapshot。CSV 里看起来像"IG 自身 JS 抛异常被全局 hook 抓到上报"。

**根因**(在 GraceChat-Earn-Android 仓库,不在 sitin-next):
`haven/src/main/java/com/harbor/prod/socialproxy/bridge/ActionDispatcher.kt:108-119` 在 WebView 注入 `window.SocialProxy` 完成前就调 `window.SocialProxy.execute(...)`。`window.SocialProxy` 此时是 undefined → 抛 `TypeError` → 被 catch 后塞入 `{success:false, error: e.message}` 上报。

server 端 `device.gateway.ts:339-341`:
```ts
const errorStr = result.error
  ? JSON.stringify(result.error)
  : (dataError ? JSON.stringify(dataError) : "Script execution failed");
```
对一个 string 调 `JSON.stringify` → `"\"Cannot read...\""` 写入 message 字段。导出 SQL 用 `message::jsonb->>'code'` 抽不到 code,显示空。

**修法**:
- A.JS 守卫:在 `ActionDispatcher.kt` 生成的 script 里加 `if (typeof window.SocialProxy?.execute !== 'function')`,返回结构化 `SCRIPT_NOT_READY` 错误码
- B.等待就绪:发 action 前先轮询 `window.SocialProxy.execute` 就绪
- C.队列拦截:`onPageFinished` 期间用 pendingActions 队列拦,注入完成后 flush

时序竞态的特征是错误形态(裸 e.message 字符串 vs 结构化 {code,message}),sitin-next 仓库穷举 `.execute(` 调用都不会 undefined,定位时不要在脚本里找。

## 2. `ELEMENT_NOT_FOUND` 误归类

**坑**:`packages/app-ins-scripts/src/instagram/actions/clickFollowButton.js` 报 `ELEMENT_NOT_FOUND` 的 1743 条里,91% 实际是 404 / 登录墙 / 5xx,只有 ~150 条是真实的"按钮 selector 没命中"。

**根因**:
- `utils.detectPageState` + `buildPageStateError` 的细分错误码方案 2026-06-25 才上线(commit `b8442188`),CSV 数据窗口 06-18 ~ 06-26 大部分时间还没有这套分流
- 即使上线后,detectPageState 的 `title === "Instagram"` 是精确匹配,漏掉 `(9+) Instagram` 等角标 title(IG 在 inbox/notif 数 > 0 时 title 带前缀角标)

**修法**:
- `utils.js:514-528` 的 title 精确匹配改为正则 `/^(\(\d+\+?\)\s*)?Instagram$/.test(title.trim())`
- 更稳的做法是**正向校验"已水合 profile"**:`title.includes("(@" + handle + ")")` — `(@handle)` 是 IG 跨语言的不变量(中文 "Instagram 照片和视频"、英文 "Instagram photos and videos"、西语 "Fotos y videos de Instagram",后缀因语言而异,但 `(@handle)` 不变)

## 3. `INVALID_PARAMS` 暴露上游 caller bug

**坑**:380 条 INVALID_PARAMS 里,153 条 URL 是 `https://www.instagram.com/`(根本没进 profile),102 条是 `/direct/inbox/`,还有 email 当 handle 用(如 `wsisler@myyahoo.com`)、含 `@` 前缀 URL 等。

**根因**:server 端 todo 调度器没校验 handle 格式,把 email/异常 handle 直接下发。

**修法**:在 `app-social-proxy-server` 上游 caller(behavior.service.ts / todo 生成处)加 handle 格式校验 `/^[a-zA-Z0-9._]{1,30}$/`,过滤掉 email、空字符串、direct 路径。

## 4. dom_snapshot 截断长度

历史值 50/100/120/300/500/2000 太严,IG profile 页 body 几百 KB,context_html 截 2000 字几乎看不到结构。2026-06-26 统一放宽到 10000(见 daily-2026-06-26 工作日志)。

进一步扩容方案(未采用,留作参考):
- full_dom 不截断 `document.documentElement.outerHTML` → ~2MB/条
- body-only + gzip + base64 → ~250KB/条
- gzip + WebSocket binary frame → ~180KB/条
- 当前 10000 截断 → ~50KB/条

[[2026-06-26]]
