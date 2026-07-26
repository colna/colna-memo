---
title: sitin-next 分支约定
date: 2026-07-23
tags: [sitin-next, reference, git, branching]
---

# sitin-next 分支约定

## 集成分支

- **当前**:`feature/sitin4.0`(2026-07-23 确认)
- **旧名**:`personal/zz/sitin4`(2026-07-09 前),已废弃,PR 别再 base 到它上

## 个人分支命名

- `personal/zz/pwa-<主题>` — app-pwa 相关
- `personal/zz/admin-<主题>` — admin 后台相关
- `personal/zz/sp-<主题>` — social-proxy 相关
- `personal/zz/fix-<主题>` — 通用修复

## PR 流程

- **base**:集成分支 `feature/sitin4.0`(不是 main)
- **身份**:colna(SSH alias `github-colna`)
- **commit scope**:强制,例如 `fix(app-pwa):`、`feat(business-minerva-users):`,否则 Lerna 全包升版
- **pre-commit**:全仓 `pnpm lint`,单包坏就全挂;当前 `business-minerva-users` 有 `no-useless-assignment` 老错阻塞
- **pre-push**:全仓 `pnpm build` + `pnpm test`,minerva 系列多个包挂
- **--no-verify**:CLAUDE.md 硬规则禁止,但用户可临时授权;不要默认绕过

## 相关

- 本仓库工作规则:`/Users/a0000/Dev/zhangzheng/sitin-next/CLAUDE.md`
- 编码规范:`/Users/a0000/Dev/zhangzheng/sitin-next/docs/coding-conventions.md`
- 金额字段单位:[[sitin-money-units]]
