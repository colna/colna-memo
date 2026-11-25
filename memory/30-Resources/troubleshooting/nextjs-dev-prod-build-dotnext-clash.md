---
title: Next dev 与 next build 共用 .next 导致 "Cannot read properties of undefined (reading 'call')"
date: 2026-11-25
tags: [nextjs, webpack, dev-server, troubleshooting]
---

# Next dev 与 next build 共用 .next 导致 "Cannot read properties of undefined (reading 'call')"

来源：super-image2 `apps/web`（Next.js 15.5.18 + webpack，2026-11-25）。

## 症状

- 浏览器满屏 Next dev overlay：`Runtime TypeError: Cannot read properties of undefined (reading 'call')`，Call Stack 全是 ignore-listed 帧（React / webpack 内部）。
- 页面白屏，但 `next dev` 进程本身活着、`GET /chat 200`，服务端日志无报错。
- 该错误来自 webpack runtime：`__webpack_modules__[moduleId].call(...)` 里 module 不存在，属于**打包产物/缓存不一致**，不是业务代码 bug。

## 根因

`.next/BUILD_ID` 和带 hash 的 chunk（`layout-cc1d7248....js`）是 `next build` 产物，而 dev server 吐出的 HTML 引用 dev 命名（`/chunks/app/chat/layout.js`，无 hash）→ 404，客户端 chunk 与 webpack runtime 对不上。

判断依据：

```bash
ls apps/web/.next/BUILD_ID          # 生产构建残留
ls apps/web/.next/static/chunks/app/chat/   # 只有 layout-<hash>.js
grep -o '/_next/static/chunks/[^\"]*\.js' /tmp/page.html   # 却是 layout.js
```

即：先在 14:45 启动了 `next dev`，15:04 又跑了一次 `next build`（或反之），两者共用同一个 `.next` 目录，dev 的模块图被生产产物覆盖/污染。

## 修法

```bash
# 1. 停掉 dev server（端口 3000）
# 2. 把 .next 挪走而不是直接删（可回滚）
cd apps/web && mv .next "$TMPDIR/super-image2-next-backup-$(date +%s)"
# 3. 重启
cd ../.. && pnpm dev            # 或 pnpm dev:web
```

验证：页面 HTML 里的 chunk 全部 200；浏览器 console 不再有 `reading 'call'`。

## 经验

- **不要在同一目录同时跑 `next dev` 和 `next build`**；要验证 build 就换目录（worktree/副本）或用 `--distDir`。
- 带 hash 的 chunk + 无 hash 的 HTML 引用，是 dev/prod 产物混用最直接的指纹。
- overlay 里全是 ignore-listed 帧时，先怀疑构建产物，别翻业务代码。
