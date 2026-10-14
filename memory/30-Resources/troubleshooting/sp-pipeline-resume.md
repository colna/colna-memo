---
title: SP.pipeline 断点恢复机制（app-ins-scripts）
date: 2026-10-14
tags: [sitin-next, app-ins-scripts, pipeline, resume]
---

# SP.pipeline 断点恢复机制

> 来源：sitin-next `packages/app-ins-scripts/src/instagram/actions/executeAction.js`（2026-10 实测 + 源码通读）
> 适用：脚本内部声明式多步骤编排（`followUserById` 为原始范例；`createPost` / `getMediaInsights` 2026-10-14 改造接入）

## 两套 pipeline 别混

| | SP.pipeline（脚本内部） | executePipeline（服务端下发） |
|---|---|---|
| 存储键 | `opts.stateKey` 自定义（如 `__createPost_pipeline`） | 固定 `__sp_pipeline_state` |
| 步骤来源 | 脚本内 inline fn | 服务端 steps 数组（引用 SP 方法名） |
| recovery 自动恢复 | **不会**直接自动恢复 | **会**（recovery 读全局 key 优先处理） |

## 恢复链路（SP.pipeline 全貌）

1. 调用经 `SP.execute(action, params, requestId)` → 写 `__sp_pending_tasks`
2. 方法内 `SP.pipeline(...)`：每步**执行前** `saveProgress(i+1)` 存 `{step, ctx, ts}` 到 stateKey
3. 步骤 fn 返回 `pending` 结果串（导航触发）→ pipeline 挂起：保留 stateKey 断点、不清
4. 页面重载 → 脚本重注入 → recovery 单步路径读 `__sp_pending_tasks` → **重调 `SP[task.action](params)`**
5. 重调时 pipeline 读 stateKey 断点 → **从断点步骤继续**（`startStep`）
6. 完成（成功/失败）→ finally 清 stateKey；`opts.onFinally` 延迟 1s 调用（不阻塞结果上报）

## 关键语义（踩坑点）

- **`saved.ctx` 不恢复进 ctx**：只用于 `validateCtx(savedCtx, currentCtx)` 校验（不一致则清断点重新开始）。**ctx 里的动态数据（uploadIds/items 等）必须能从调用参数 + 页面现状重建**——所以导航步应放最前，导航后的步骤重跑时自愈。
- **断点=下一步**（`saveProgress(i+1)` 在执行前）：语义假设「步骤要么完整执行，返回 pending 挂起；要么整步跳过」——**长步骤执行中页面被杀（非导航）→ 恢复时会跳过该步**，需在设计上容忍（如发布步检查前序产出缺失则报 `PIPELINE_STALE`）。
- **pending 挂起时 onFinally 也会被 setTimeout 调用**（1s 后）：onFinally 里做副作用（跳转等）必须加 guard（如 `ctx.finished`），否则会和导航互踩。
- **ttl**：默认 120s，按最慢步骤耗时放大（createPost 用 600s，覆盖转码轮询 62.5s+上传）。读取时 `now - saved.ts > ttl` 才过期（ts 每步刷新）。
- **裸调用（不经 SP.execute）没有自动恢复**：无 pending 记录 → 没人重调 → 断点留在 sessionStorage；**再次手动调用同方法**即可从断点续（这就是 `getMediaInsightsResume` = 转发主方法的原理）。
- **返回值约定**：fn 返回 `undefined` 继续；返回 string（result JSON）立即透传（含 buildError 错误码——想保留错误码精度就 `return utils.buildError(...)` 而不是 throw）；throw → `EXCEPTION` 统一错误。
- **blob 等大对象不能进 ctx**（JSON 序列化到 sessionStorage）：放模块级缓存（页面内有效），恢复后按需重新下载（分片接口有缓存，重下廉价）。

## 实例

- `followUserById`：导航步（pending 挂起）→ 后续步骤目标页执行；ctx 只放 username。
- `getMediaInsights`：`进入 Insights 页`（pending）→ `抓取报告数据`；`xxxResume` = 再次调用主方法。
- `createPost`：`下载素材 → 上传素材 → 发布帖子`；素材/解析缓存走模块级 `blobCache`/`videoParseCache`（onFinally 清理）。
