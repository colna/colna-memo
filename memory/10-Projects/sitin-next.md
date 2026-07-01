---
title: sitin-next
date: 2026-06-25
tags: project, sitin, monorepo, nextjs, pnpm, turborepo
---

# sitin-next

Sitin 平台主 monorepo,pnpm workspace + Turborepo 管理。

## 基本信息

- 仓库:`git@github-colna:presence-io/sitin-next.git`
- 路径:`/Users/user/Dev2/zhangzheng/sitin-next`
- 当前分支:`feature/sp`
- 环境:Node.js >= 20,pnpm >= 10

## 技术栈

monorepo(pnpm workspace + Turborepo + lerna),含多个可部署应用与共享包(`packages/`)。带自有 `CLAUDE.md` 与 `skills-lock.json`。

## 常用命令

```bash
pnpm install   # 首次或 lockfile 变更后
pnpm build     # 构建所有包
pnpm dev       # 启动所有开发服务
pnpm lint      # 提交前必须通过
pnpm test      # 运行测试
```

## 已完成需求

### 脚本错误日志 — 流式 CSV 导出 + 日期筛选(2026-06-30)

- **页面**:Minerva admin `/social-proxy/script-errors`(脚本执行错误日志)。
- **需求**:①修大数据量导出报错(旧实现 `error-logs?pageSize=140909` 一次性 `findMany` 拉爆内存,domSnapshot 巨大;140,949 行 / 全量 ≈1.2GB);②表格加日期范围筛选,默认最近 7 天;③导出跟随所选日期;④`target_html` 不能去掉(调试用);⑤前端加进度 Modal(MB 计数 + 可取消)。
- **后端**(`app-social-proxy-server`):新增 `GET script/error-logs/export`(必须声明在 `error-logs/:id` 之前,否则被 `:id` 捕获)。`streamCsv()` 游标分页(`orderBy [createdAt desc, id desc]` + `cursor/skip:1`),每批 2000 条,边取边写 Express 响应流,内存恒定、任意体量可导。BOM + `\r\n`,`target_html` 保留在 CSV。
- **前端**(`app-minerva-web`):`RangePicker` 默认近 7 天;导出用 File System Access API(`showSaveFilePicker` → `createWritable`)+ fetch reader 直接流式写盘,浏览器内存也恒定;非 Chromium 走 blob fallback。进度按字节计(流响应无 Content-Length,行数因 target_html 内嵌换行不可靠),Modal 显示已写入 MB + 取消(AbortController → `writable.abort()` 丢弃半成品文件)。
- **数据边界**:导出放 social-proxy-server 而非 minerva-server —— `script_error_log` 表只在 social-proxy 的 Prisma;minerva-server 是 BFF,`/api/social-proxy/*` 用 `@fastify/http-proxy` 流式透传(`reply.from` 不缓冲),故流式 CSV 经它透传内存依旧恒定。放 minerva-server 反而要多一跳或跨库,更糟。
- **PR / 分支**:
  - 特性分支 `personal/zz/script-errors-export`(从 `feature/admin@261ef60b` 切),2 个提交 `d81bd05b`(后端)+ `f2a3b11a`(前端)。
  - **PR #491** → base `feature/admin`:<https://github.com/presence-io/sitin-next/pull/491>
  - 上测试环境:cherry-pick 这 2 个提交到 `release/test-admin` 并 push(`c71e9276..b31e31c0`)。**不用 merge**:该分支基于旧的 261ef60b,feature/admin 从未合进 release/test-admin(分叉点 3d3de7e0),merge 会拖入整段 feature/admin delta 并产生回退型冲突(如 `guild-queue.ts` 会用旧版覆盖新 import 破坏构建)。

## 备注

- 详细应用清单 / 线上域名见仓库 README「可部署应用」表。
- 项目自带 `docs/` 与 `deploy/`。
