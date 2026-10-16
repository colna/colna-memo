---
title: Koa 3 服务实战踩坑（koa-agent）
date: 2026-10-14
tags: [koa, typescript, zod, node, koa-agent, troubleshooting]
---

# Koa 3 服务实战踩坑（koa-agent）

来源：`/Users/colna/TEST/agent-demo`（koa-agent，Koa 3 + zod 4 + vitest + tsx，2026-10-14）。姊妹条目：[agentscope-ts-sdk-gotchas](agentscope-ts-sdk-gotchas.md)（Koa 3 无自动校验 → 手工 zod；SSE `ctx.respond=false` 原生写）。

## 坑与结论

1. **隐式 404 上赋 `ctx.body` 会被拉回 200**：未匹配路由时 Koa 状态是默认 404（`_explicitStatus=false`），直接 `ctx.body = {...}` 会触发 body setter 把状态重置成 200。修法：先 `ctx.status = status`（显式）再赋 body。统一错误中间件的兜底分支写成：`const status = ctx.status; if (ctx.body == null && status >= 400) { ctx.status = status; ctx.body = envelope; }`。
2. **`@koa/router` allowedMethods 的 405 只设状态、不设 body**：响应由 Koa 兜底为纯文本 `Method Not Allowed`（非 JSON）。同上，统一错误中间件对「status ≥ 400 且 body 为空」补 JSON 信封才能兜住。
3. **手动 `process.emit('SIGTERM')` 不自动带信号名参数**：真实信号 Node 以 `emit(signal, signal)` 调用监听器；测试里手写 `emit('SIGTERM')` 监听器收到 `undefined`。修法：`process.emit('SIGTERM', 'SIGTERM')`。
4. **优雅退出的可测写法**：`setupGracefulShutdown(running, log, exit = process.exit)` 注入 `exit`，返回 `dispose()` 摘监听器；测试用 `vi.fn()` 替身 + `vi.waitFor` 断言「关闭完成 + exit(0)」，避免 vitest 进程被真实 `process.exit` 杀掉。
5. **zod v4 用顶层格式**：`z.url()` / `z.coerce.number()` 在 4.6.x 可用；校验 `process.env` 时用默认 strip（不要 `.strict()`，env 里全是无关变量）；报错里 `issue.path.join('.')` 输出字段名友好。
6. **TS NodeNext ESM + 构建**：源码相对导入写 `.js` 后缀（tsx / vitest / tsc 三边都能解析），`tsconfig.build.json` 显式 exclude `**/*.test.ts` 与测试基建目录，`node dist/index.js` 才能直接跑。
7. **入口自动执行守卫**：`import.meta.url === pathToFileURL(process.argv[1]).href` 判断是否直接运行（tsx 与 node dist 都成立），被测试 import 时不会触发 `main()`。
8. **CORS 预检的「测试盲区」**：`Access-Control-Allow-Methods` 没列全（如只放行 GET/POST，忘了 DELETE）时——**Node 集成测试用 `fetch` 不走 CORS，全绿**；浏览器端预检被拦、请求静默失败（表现：前端"删不掉"，服务端无日志）。修法：方法清单与路由实际使用对齐（GET/POST/DELETE/OPTIONS），并在测试里对预检响应头做断言（`access-control-request-method: DELETE`）。教训：涉及浏览器专属行为（CORS/预检/存储）的功能，集成测试通过 ≠ 浏览器可用。
