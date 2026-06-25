---
title: sitin-demo-webapp
date: 2026-06-25
tags: project, sitin, monorepo, nextjs, vite, turborepo
---

# sitin-demo-webapp

Sitin Webapp,基于 Turborepo 的 monorepo(包名 `sitin-webapp`),pnpm 包管理。

## 基本信息

- 仓库:`git@github-colna:presence-io/sitin-demo-webapp.git`
- 路径:`/Users/user/Dev2/zhangzheng/sitin-demo-webapp`
- 当前分支:`main`
- 环境:Node.js >= 18,pnpm 9.x

## 项目结构

```
apps/
  sitin-official/   # 官网 (Next.js)
  sitin-web/        # Web 应用 (Next.js)
  sitin-pwa/        # PWA 应用 (Vite + React)
  sitin-video/      # 视频应用 (Vite + React)
  sitin-worker/     # Worker 应用 (Vite + React)
  growth/           # 增长页面 (静态)
packages/
  components/       # 共享 UI 组件
  utils/            # 共享工具函数
  http/             # HTTP 及 proto 定义
  eslint-config/    # 共享 ESLint 配置
```

## 备注

- 与 `sitin-next` 同属 Sitin 平台,关系待确认(demo / 旧版?)。
- 根目录含 `heyhru-server-*.json`(疑似服务账号密钥,勿外传)。
