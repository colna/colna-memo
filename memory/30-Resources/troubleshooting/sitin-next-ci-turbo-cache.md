---
title: sitin-next CI — turbo 缓存不生效 & minerva 双重构建
date: 2026-06-29
tags: [sitin-next, jenkins, turbo, ci, docker, troubleshooting]
---

# sitin-next CI — turbo 缓存不生效 & minerva 双重构建

## 架构(谁在哪)

- 流水线逻辑在**共享库** `presence-pipeline`(= `jenkins-shared-lib` 的 `aliyun-next` 分支),不在 jenkins-projects。
- `jenkins-projects` 只放 `projects/<proj>/{dev,prod}.yaml` + `hooks/`。webhook 自动据 YAML 建/同步 Jenkins job。
- 阶段:`Build`(全局 `build.cmd`,跑一次)→ `Pack`(逐服务,`hooks.pack`)→ `Publish` → `Deploy`。
- Dockerfile 在**各源仓库**;`projects/sitin-next/hooks/docker-pack.sh` 用 `packages/app-${svc}/Dockerfile` 普通 `docker build`(为透传 `--build-arg APP_ENV`)。

## 双重构建根因(只有 minerva-server 编两次)

- `build.cmd: pnpm install && pnpm run build`(**无 filter**)在宿主全量编**所有包**(含 minerva-server、minerva-web…)。
- 两个 Docker 服务 Dockerfile 策略**相反**:
  - `social-proxy-server`:**信任宿主产物** —— `COPY packages/app-social-proxy-server/build/`、`COPY minerva-schemas/dist/`,容器内**不**跑 turbo。→ 编 1 次(宿主)。
  - `minerva-server`:**不信任** —— 容器内 `RUN pnpm exec turbo run build --filter=@heyhru/app-minerva-server...` 重编。→ 编 2 次(宿主 + 容器)。
- `minerva-web` 不在 services、非 minerva-server 依赖 → 本流水线**不部署**,却被无 filter 的全量 build 白编。

## 缓存不生效根因

turbo 缓存纯本地 `.turbo`(gitignore):
1. **容器内**:Dockerfile 没有 `RUN --mount=type=cache,target=.../.turbo`,且 `.turbo` 不进构建上下文 → 每次冷缓存全量。
2. **宿主**:`.turbo` 在工作区,流水线末尾 `cleanWs` 清掉 → 下次冷。
3. **无远程缓存**:turbo.json 无 `remoteCache`,无 `TURBO_TOKEN/TEAM/API`。

## 修法(2026-06-29,最小、无新基建)

- **jenkins-projects**(`fix/sitin-next-turbo-cache`):
  - `dev/prod.yaml` 的 `build.cmd` → `pnpm exec turbo run build --cache-dir=/data/jenkins/turbo-cache/sitin-next-{dev,prod} --filter=@heyhru/app-static-pages --filter=@heyhru/app-social-proxy-server`。收窄(去掉 minerva → 不再宿主重复编 = 修双重构建)+ `--cache-dir` 指工作区外持久路径(躲过 cleanWs)。
  - `hooks/docker-pack.sh` 加 `DOCKER_BUILDKIT=1`。
- **sitin-next**(`personal/zz/ci-turbo-cache`,从 feature/admin):`packages/app-minerva-server/Dockerfile` 给 `turbo run build` 加 `--mount=type=cache,target=/app/.turbo`、三个 `pnpm install` 加 pnpm store mount。cache mount 不进镜像层。

## 前提与边界

- 依赖 Jenkins 为**固定持久节点**(docker daemon/磁盘持久)→ cache-dir 与 BuildKit cache mount 才能跨构建存活。若上**弹性 agent**,本地缓存失效 → 须 **Turbo Remote Cache**。
- 宿主缓存(cache-dir)与 Docker 缓存(BuildKit mount)是**两套独立缓存**,不共享;同一包被两边各编各缓存。全 monorepo 一套统一缓存只有 Remote Cache 能做到。
- 全 20 个 jenkins 项目里只有 **sitin-next + sitin-monorepo** 用 turbo;其余 Gradle。sitin-monorepo 已删外层 build.cmd(无双重构建),但其各服务 Dockerfile 同样缺 cache mount,要生效需单独同样处理。

## 已落地与实测(2026-06-29 验证通过)

三轮改动,minerva 单次构建(暖)~6min → ~3.5–4min:

| 优化 | 手段 | 实测 |
|---|---|---|
| **A turbo 缓存 + 修双重构建** | 宿主 `--cache-dir`(躲 cleanWs)+ Docker BuildKit `--mount=type=cache,target=/app/.turbo`;`build.cmd` 收窄去掉 minerva | turbo 74s(冷)→33s(暖);minerva 不再宿主+Docker 双编 |
| **B-1 精简镜像** | `pnpm --filter=app-minerva-server deploy --prod --legacy /app/deploy` + `package.json "files":["build","prisma"]`;runner 扁平 `COPY /app/deploy ./` + `CMD node build/index.js` | **导出 110s→13s、COPY node_modules 52s→1.4s** |
| **C 类型安全** | tsconfig 排 test;删 prisma symlink(import +1 `../`);去 `{ tsc \|\| true; }` | tsc 0 错;堵住坏类型静默上线 |

**真瓶颈认知**:导出+拷贝(162s,占 45%)= node_modules 体积;不是 install(~42s)、不是 turbo。`pnpm deploy` 治本。

**关键坑**:
- `pnpm deploy` 下 build/ 被 gitignore → 必须 `files:["build","prisma"]` 显式纳入,否则 deploy 不带,容器起不来。prisma musl 引擎随 build/ 进 deploy(`binaryTargets` 已含 `linux-musl-openssl-3.0.x`),dev-213 Pod 实证能连。
- 扁平化后 `PKG_ROOT = build/.. = /app`,`.env` 放 `/app/.env`、`CMD node build/index.js`。
- prisma symlink 的 TS2322 **只在 alpine 复现**(macOS realpath 自动合一),本地不是可靠 oracle,靠 CI 验。
- 去 `\|\| true` 的潜在类型错**分支+OS 特定**,本地穷举不了;decision-executor 真错只在 release/test-admin(带群发代码)出现并修复(用 `isDispatchHandled()`)。

**验证**:#213 tsc 0 错 + dev-213 Pod `Server listening :3000`、0 error。PR #481(colna)。
