---
title: commerce-frontend 项目概览
date: 2026-11-18
tags: project, commerce-frontend, monorepo, frontend
---

# commerce-frontend

E-commerce 前端 monorepo，2026-11-18 创建。

- 仓库：<https://gitlab.sitinai.com/e-commerce/commerce-frontend>
- GitLab project id：735
- 本地路径：`/Users/colna/WORK/commerce-frontend`
- 默认分支：`main`

## 目录

```text
apps/admin/            # 管理后台（React 19 + AntD 5 + Umi Max）
apps/sp-scripts/       # Instagram 自动化脚本（esbuild 构建 + 上传部署）
packages/shared/       # 跨端类型与工具
```

## 常用命令

```bash
pnpm install
pnpm dev
pnpm lint
pnpm typecheck
pnpm test
pnpm build
```

后台默认 <http://127.0.0.1:5173>，API base 用 `UMI_APP_API_BASE` 配置。

## 设计决策

- 参考 `sitin-next`：pnpm workspace + Turborepo + 共享包。
- admin 用 Umi Max + React 19 + AntD 5，路由和运行时约定与 sitin-next 对齐。
- SP 脚本同时产出 `dist/sp.iife.js`（页面注入）和 `dist/sp.mjs`（ESM 消费）。
- `apps/sp-scripts` 与 admin 同级，作为独立 app 管理。
- Instagram 脚本同步自 `sitin-next` `feature/sp`，当前基线 `70e6ad68`，构建产物为 `dist/instagram/{automation,bootstrap,login}.js`。
- SP 脚本部署只上传 esbuild 产物，使用 `SP_DEPLOY_API_URL` 调用上传接口；接口待定，当前 multipart 字段为 `file` + `metadata`。
- 包 scope 统一 `@ecommerce/*`。
- CI 分 `verify → build → deploy`：任意分支跑 verify；`release/test-sp` / `release/online-sp` 合入后自动部署 SP 脚本；admin 目前是 demo，不接部署。

## 当前状态

- 初始提交：`22becdf feat: scaffold commerce frontend monorepo`。
- admin Umi 迁移：`4edefd9 refactor(admin): migrate to umi max`。
- 测试：shared 2、sp-scripts 5、admin 2，共 9 passed。
- GitLab pipeline `8335` 已 success（main，含 SP 脚本迁移与 release 部署 CI）。
- SP 脚本迁移与部署链路已在本地完成验证，待接口确认后提交/推送。
- 尚未接真实后端、登录态、权限与部署。

## 下一步

1. 接电商后端 API 与登录/鉴权。
2. 补 admin 的商品/订单真实 CRUD。
3. 把具体的 SP 平台 action 按目录落到 `packages/sp-scripts/src`。
4. 确认 SP 上传接口并配置 `SP_DEPLOY_API_URL` / `SP_DEPLOY_TOKEN`。
