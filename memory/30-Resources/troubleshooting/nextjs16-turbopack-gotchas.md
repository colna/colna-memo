---
title: Next.js 16 / Turbopack 实战踩坑（koa-agent web）
date: 2026-10-14
tags: [nextjs, turbopack, monorepo, pnpm, tailwind, troubleshooting]
---

# Next.js 16 / Turbopack 实战踩坑（koa-agent web）

来源：koa-agent Phase 6（`apps/web`，Next.js 16.4.0 + Tailwind 4 + pnpm monorepo，2026-10-14）。姊妹条目：[koa-server-gotchas](koa-server-gotchas.md)、[langgraph-js-streaming](langgraph-js-streaming.md)。

## 坑与结论

1. **web 侧相对导入不能带 `.js` 扩展名**：Turbopack 按 bundler 解析，不认 NodeNext 风格的 `./sse.js → sse.ts` 映射，构建报 `Module not found: Can't resolve './sse.js'`；而 server 侧（tsc NodeNext 产物直跑）**必须**带。同一 monorepo 两种约定 —— 写代码时先看所在包的 tsconfig。
2. **`LayoutProps<"/">` 等路由类型是生成物**：由 `next dev` / `next build` / `next typegen` 在 `.next/types` 生成；干净检出直接 `tsc --noEmit` 会报类型缺失。web 的 typecheck 脚本应为 `next typegen && tsc --noEmit`。
3. **biome 会扫到 `.next` 生成文件**（`.next/types/validator.ts` 含 `any` 会报错）：在 `files.includes` 加 **文件夹级**排除 `!apps/**/.next`（biome 的 useBiomeIgnoreFolder 规则要求这种写法，不要写 `/**` 后缀）。
4. **create-next-app 会在子目录生成嵌套 `pnpm-workspace.yaml`**（内有 `ignoredBuiltDependencies: [sharp, unrs-resolver]`）：在 monorepo 里会让该子目录被当作独立 workspace root，导致 pnpm 行为混乱。正确做法：把内容合并到根 `pnpm-workspace.yaml`，删除嵌套文件。
5. **POST + SSE 不能用浏览器 `EventSource`**：它只支持 GET、不能带自定义头/请求体。浏览器端应使用 `fetch` + `ReadableStream` 逐段读取并自解析 SSE 帧（半帧要用 buffer 拼接）；跨域需 server 端 CORS 放行（含 OPTIONS 预检直接回 204）。
6. **`reactCompiler: true` 与 lint 的「signal 依赖」**：编译器接管记忆化（写纯组件即可，不必手工 useMemo/useCallback）；但 biome 的 `useExhaustiveDependencies` 会把「effect 只把 state 当触发信号、不在体内读取」误报为多余依赖，需 `// biome-ignore ...` 显式说明原因。
7. **Tailwind 4 接入 Turbopack 走 loader**：`next.config.ts` 的 `turbopack.rules["*.css"].loaders: ["@tailwindcss/turbopack"]`（不需要 postcss 配置）；设计令牌用 CSS `@theme`（含 `@keyframes`）定义，utility 自动生成。

## 复现命令

```bash
cd /Users/colna/TEST/agent-demo
pnpm --filter @koa-agent/web typecheck   # next typegen && tsc --noEmit
pnpm --filter @koa-agent/web test        # 8 passed（全离线）
pnpm --filter @koa-agent/web build       # Next 16.4.0 Turbopack
```
