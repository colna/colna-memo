---
title: sitin-prds
date: 2026-10-09
tags: [project, sitin, prd, prototype, gitlab, k8s]
---

# sitin-prds

Sitin 交互需求原型静态站（PRD Studio）。**一个需求 = 一个文件夹 = 一个 path**；文件夹内原型是创始人 → 产品 → 设计 → 前端 → 后端 → 数据共同迭代的单一事实来源。

## 基本信息

- 仓库：`git@gitlab.sitinai.com:ai/sitin-prds.git`（内网 GitLab，账号 zhangzheng / @zhangzheng；接入见 [gitlab-sitinai-access](../30-Resources/troubleshooting/gitlab-sitinai-access.md)）
- 本地：`~/WORK/sitin-prds`（2026-10-09 clone，main）
- 线上：<https://prds.sitin.ai>（另有 prds.sitinai.com）
- 纯静态、无构建、无框架；预览 `npx serve .` 或直接 open 原型 HTML。

## 目录结构

```
index.html                需求列表入口（每需求一张卡片）
sitin-4-2/                需求文件夹（= 一个 path）
  discover-nearby-online.html    主原型（结构+交互 JS，3442 行 / 202KB）
  discover-nearby-online.css     样式（设计师维护，4841 行 / 183KB）
  discover-nearby-online.data.js 产品说明 panel 数据（FEATURES，693 行）
  interaction-logic.html        交互逻辑规格（辅助页）
  interaction-showcase.html     交互动效展示（辅助页）
  modal-state-gallery.html      弹窗 & 状态图鉴（辅助页）
  assets/                       设计静态资源（云/礼盒/金币等 PNG+SVG）
designs/                  tokens.css + design-spec.md（SITIN 4.0 PWA 设计 token，从 Figma 规范板抽取）
ci/                       acr-login.py（换 ACR 临时凭据）、update-gitops-tag.py（写 GitOps tag）
deploy/                   Dockerfile（nginx:alpine）+ default.conf（复刻旧 vercel 路由 + /healthz）
.opencode/skills/figma-to-prototype/   本仓库自带设计还原 skill（SKILL.md + lessons.md + png-sample.js）
vercel.json               仅留档（旧 Vercel 路由，集群部署不用）
AGENTS.md 规范手册 / rules.md 角色职责 / PRODUCT.md / DESIGN.md / CLAUDE.md
```

## 协作模型（rules.md）

按「创始人 → 产品 → 设计 → 前端 → 后端 → 数据」依次补 `FEATURES`（`<需求>.data.js`）：

| 角色 | 字段 |
|---|---|
| 创始人 | `name` / `idx` / `pitch` / `anchor` + 左侧交互动效 |
| 产品 | `details` |
| 设计 | `.css`、`assets/`、设计 token（不碰业务字段） |
| 前端 | `api.url`（METHOD /path）+ `desc` + mock 入参出参 |
| 后端 | `api.input` / `output` 完整契约 |
| 数据 | 埋点 / 数据表 / 指标口径 |

## 部署链路

- **主链路（现役）**：merge `main` → Jenkins `deploy-servers/sitin-prds/{dev,prod}` 构建 → ACR `frontend/sitin-prds` → 写 tag 到 GitOps `jenkins-k8s-values`（`project/sitin-prds/<env>`）→ ArgoCD 从 `dora-k8s-config` 同步。
- **新增（2026-10-09）**：`.gitlab-ci.yml` —— push 到 **`feature/sitin-*` 分支自动** kaniko 构建 → 推 ACR → `ci/update-gitops-tag.py` 写 tag → ArgoCD 滚动固定站。纯 GitLab CI（不经 Jenkins），新分支名命中即生效。CI Variables：`ACR_AK` / `ACR_SK` / `GITOPS_TOKEN`。
- nginx 配置复刻旧 Vercel 行为：`cleanUrls` + `/sitin-4-2` rewrite 到 `discover-nearby-online.html`。

## 分支规范

`personal/<名字>`（个人）→ PR → `feature/<需求id>`（集成分支，合入触发自动部署）→ `main`（只放共享文件）。origin 上有大量 `personal/*` 与 `feature/sitin-4.2|4.3` 分支。

## 当前状态（2026-10-09）

- **sitin-4.2 已发布**（`feature/sitin-4.3` 分支上有「4.2 卡片置灰并提示已发布」的提交 84f0e81）。
- **sitin-4.3 已开局**：从 sitin-4-2 复制开新版本并注册路径（0cbaf99）；PWA 4.3 C 线（照片线/活跃等级/安卓登录页脚本）另在 sitin-next + app-ins-scripts 推进。
- sitin-4-2 含 11 个 feature：注册定位 / 附近在线 / 滑动匹配 / Quick 会话 / My Picks / 任务提现 / 上线赚钱 / 真人认证 / 用户分层·权益 / 约会牵线 / 授权解耦；样式按 Figma 还原过多轮（commit `dbd95bf` 等）。
- main 最新 commit `79348df`（合入 auto-deploy CI）。

## 约定与坑

- **Git 身份冲突（待用户裁决）**：仓库 `CLAUDE.md` 要求 commit 用 `max-presence` 身份（`-c user.name="max-presence" -c user.email="127914917+max-presence@users.noreply.github.com"`），且声明「auto commit + push 无需确认」；与全局规则「身份统一 colna / 禁止自动推」冲突，实施前须确认取哪边。
- 原型是手工大文件（sitin-4-2 主原型 200KB+）→ **手术式修改**，不重排缩进、不大段重写。
- 需求文件夹内文件名保留**描述性命名**（不叫 index.html）；新增需求要同步 `index.html` 卡片 + nginx rewrite。
- Figma 还原走仓库自带 skill `figma-to-prototype`（含「拿不准就采样」`png-sample.js`、细节对齐循环；只碰 `.css`/`.html`/`assets/`，不碰 FEATURES 业务字段）。
- 界面文案可用英文；规范/注释/文档用中文。

## 相关笔记

- [gitlab-sitinai-access](../30-Resources/troubleshooting/gitlab-sitinai-access.md) — 内网 GitLab 接入
- [sitin-next](sitin-next.md) — 平台主 monorepo（sitin-4.3 PWA 线在此推进）
