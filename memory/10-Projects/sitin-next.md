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

## 备注

- 详细应用清单 / 线上域名见仓库 README「可部署应用」表。
- 项目自带 `docs/` 与 `deploy/`。
