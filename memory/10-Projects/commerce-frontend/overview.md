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
apps/admin/            # 管理后台（React 19 + AntD 5 + Vite 7）
packages/sp-scripts/   # Social Proxy 注入脚本（TS + esbuild）
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

后台默认 <http://127.0.0.1:5173>，API base 用 `VITE_API_BASE` 配置。

## 设计决策

- 参考 `sitin-next`：pnpm workspace + Turborepo + 共享包。
- admin 用 Vite + React 19 + AntD 5，不引入 Umi，保持轻量。
- SP 脚本同时产出 `dist/sp.iife.js`（页面注入）和 `dist/sp.mjs`（ESM 消费）。
- 包 scope 统一 `@ecommerce/*`。
- CI 只做 `install → lint → typecheck → test → build`，部署流水线后续再接。

## 当前状态

- 初始提交：`22becdf feat: scaffold commerce frontend monorepo`。
- 测试：shared 2、sp-scripts 5、admin 2，共 9 passed。
- GitLab pipeline `8271` pending，等待可用 runner。
- 尚未接真实后端、登录态、权限与部署。

## 下一步

1. 接电商后端 API 与登录/鉴权。
2. 补 admin 的商品/订单真实 CRUD。
3. 把具体的 SP 平台 action 按目录落到 `packages/sp-scripts/src`。
4. 配 CI runner 与静态资源部署。
