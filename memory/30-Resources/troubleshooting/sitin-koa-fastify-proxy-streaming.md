---
title: sitin-next Koa/Fastify 反代流式响应
date: 2026-07-03
tags: [sitin-next, koa, proxy, streaming, csv-export, troubleshooting]
---

# sitin-next Koa/Fastify 反代流式响应

针对 `packages/app-minerva-server` 里 `/api/social-proxy/*` 反代 `app-social-proxy-server` 时"上游流式响应被反代层全缓"这类问题。

## 部署映射(先牢记再排查跨系统 bug)

sitin-next 单一 monorepo,但至少两个独立部署:

- `feature/admin` → `release/online-admin` → 部署 admin 前端(`app-minerva-web`)+ minerva-server(Fastify 外壳 + Koa 子进程,含反代 `/api/social-proxy/*`)
- `feature/sp` → `release/online-sp` → 部署 `app-social-proxy-server`(NestJS,真正的业务后端)

排查跨系统问题时,先分别 `git log --oneline origin/release/online-admin -5` / `... online-sp -5`,别只看某个 feature 分支的 controller,否则会误判"端点缺失"。

## 反代文件位置

`packages/app-minerva-server/src/_koa-perm/routes/social-proxy/index.ts`(挂在 Koa `/api/social-proxy` 前缀下,由外层 Fastify `@fastify/http-proxy` 转到 Koa 子进程)。

## 坑 1:`await res.text()` 直接抵消上游流式

原写法:

```ts
const res = await fetch(url, ...);
ctx.status = res.status;
const ct = res.headers.get("content-type") || "";
if (!ct.includes("application/json")) {
  ctx.body = await res.text();   // ← 全缓,上游流式 CSV 白做
  return;
}
```

问题:

1. `res.text()` 要等上游把最后一个字节吐完才 resolve。
2. Koa middleware return 之前不会给下游发响应头 → Cloudflare origin 端一直等不到字节 → 超时后回 502 / 524。
3. GB 级 / 长时间窗口的导出必挂,小结果集能通过(误判"偶尔失败")。

**修法**:直接把 WHATWG ReadableStream 转成 Node Readable 给 `ctx.body`。Koa 会走 pipe。

```ts
import { Readable } from "node:stream";
import type { ReadableStream as WebReadableStream } from "node:stream/web";

if (!ct.includes("application/json")) {
  ctx.set("Content-Type", ct);
  const cd = res.headers.get("content-disposition");
  if (cd) ctx.set("Content-Disposition", cd);
  const cache = res.headers.get("cache-control");
  if (cache) ctx.set("Cache-Control", cache);
  ctx.body = res.body
    ? Readable.fromWeb(res.body as WebReadableStream<Uint8Array>)
    : null;
  return;
}
```

Node >= 18 支持 `Readable.fromWeb`;Node 20+ 才 stable-mark,但 18 也能跑。

## 坑 2:`ctx.body = "..."`(string)会让 Koa 强制 `text/plain`

赋 string 给 `ctx.body`,Koa 自动 set `Content-Type: text/plain; charset=utf-8`,把上游的 `text/csv` 覆盖掉。反代场景要**先 `ctx.set(...)` 再赋值**;或者根本别赋 string,直接挂 stream。

顺带:string 赋值时 `Content-Disposition` 也不会自动透传——反代要显式 forward。

## 坑 3:排查"大响应超时"用窄时间窗 curl

不确定是端点错还是响应过大 → 用两次 curl 隔离:

```bash
# 窄窗口(1 分钟)—— 命中极少
curl -sS -i '.../error-logs/export?startDate=<T>&endDate=<T+1min>' -H 'Authorization: ...'

# 宽窗口(7 天)—— 生产场景
curl -sS -i '.../error-logs/export?startDate=<7d ago>&endDate=<now>' -H 'Authorization: ...'
```

- 窄 200 / 宽 502 → 大小/时长问题(反代不流式或后端慢查询)。
- 全部 404 / 500 → 路由或鉴权问题。
- 全部超时 → 上游服务本身挂了。

## 坑 4:前端 File System Access 流式写盘依赖后端 + 反代都流式

`streamExportScriptErrorLogs`(`app-minerva-web/src/pages/SocialProxy/api.ts`)用 `showSaveFilePicker` + `WritableStream` 边收边写。链路上任何一环全缓(后端 `res.write` 变 `res.send(bigString)`、反代 `res.text()` 全缓),前端"流式写盘"就只是最后一步流式,前面依然会 OOM/超时。三段都得流式才真流式:

```
NestJS streamCsv(cursor 分批 res.write)
  → Koa 反代(Readable.fromWeb pipe)
    → Fastify http-proxy(默认 pipe)
      → Cloudflare
        → browser FSA writable
```

## 相关

- 修 PR:sitin-next#522(2026-07-03)
- 上游 CSV 实现:sitin-next#512(`release/online-sp` HEAD `1d523b5e`)
- 前端导出 UI:sitin-next#491(`release/online-admin` HEAD)
- pre-push 坑参考 [sitin-next-push-prepush](sitin-next-push-prepush.md)
