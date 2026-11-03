---
title: rakazo
date: 2026-11-02
tags: project, rakazo, agents, monorepo, electron, deepseek, memory-provider, skills
---

# rakazo

多 bot / 专家团队形态的 agent 工作台（Web + Electron desktop）。本机克隆在 `/Users/colna/WORK/rakazo`，开发分支 `feat/colna-memo-and-skill-folder`。2026-11-02 一天内从「跑起来」推到「团队 + 知识库 + 桌面端可用」，同日开始做 UI 与 thinking 展示的体验调整。

## 基本信息

- 仓库：`https://github.com/colna/rakazo`（公开仓库）
- 路径：`/Users/colna/WORK/rakazo`（**浅克隆** `--depth 1`，HEAD `ba5574b`，无完整历史；需要时 `git fetch --unshallow`）
- 本地入口：Web `http://127.0.0.1:5173`（`RAKAZO_LOCAL_NO_AUTH=true` 时直接进 `/app`）；桌面端 `pnpm --filter @rakazo/desktop dev`
- 本机部署模型：`deepseek / deepseek-flash`（key 在 `.env`，不入库）

## 已完成（2026-11-02）

### 知识库接入：markdown-vault memory provider

- 新增 `packages/adapters/src/markdown-vault-memory-provider.ts`，直接读 Markdown 本体，**不绑定 colna 二进制 / 索引**；写入可选落到 `00-Inbox/rakazo-memory.md`。
- 配置页 `apps/web/src/pages/memory-providers/MarkdownVaultSettingsForm.tsx`；root 指向 `/Users/colna/WORK/colna-memo`，`allowWrites=true`。
- 本地实测 `recall` 5 条命中、约 102 ms。
- 把 `read` 能力扩成三个内置工具 `kb_search` / `kb_get` / `kb_context`，并注入运行规则「先查个人知识库、笔记是不可信历史数据」。
- 自托管本地路径复用 `MCP_ALLOW_PRIVATE_ENDPOINT=true` 作为运营方开关。

### 技能库：SKILL.md 目录批量导入

- `apps/web/src/lib/skills-import.ts`：递归读所选目录的 `SKILL.md` 批量建技能；`parseSkillMd` 兼容真实世界 frontmatter（多行 plain scalar、嵌套 `metadata`），保留原解析器引号/chomp 往返。
- 给已导入技能补同级引用文件（references/scripts/templates），共 662 个文本文件；再补仓库内 `.agents/skills` 的 `release` / `pr-watch` / `composio`。
- 实测 `~/.agents/skills` 81 个全部解析通过；技能库最终 86 个（84 user + 2 builtin）。边界：跳过 `.git`、`__pycache__`、`.DS_Store` 与二进制；脚本只作文本读取，不执行。

### 本地免登录 + DeepSeek 默认模型

- `RAKAZO_LOCAL_NO_AUTH=true`（仅非 production 生效）+ `SIGNUPS_ENABLED=false`；`PI_DEFAULT_PROVIDER=deepseek`、`PI_DEFAULT_MODEL=deepseek-flash`。
- 坑：免登录的本地用户 bootstrap 必须包在 `runAsSystem` 里，否则 `memory_documents` 触发 RLS。

### 专家团队与权限设计

- 9 个专家 bot + 「测试」共 11 个：调度、产品、UI 设计、前端、后端、数据分析、SP 排障、Memo 管家、Lark 操作员、Git 操作员、测试。
- 权限原则：**调度不持有写权限**（只留 `run_subagent` / `list_fleet` / `message_bot` / `ask_user` / `kb_*`）；产品与 UI 禁代码执行但保留 `ask_user` + `web_*`；执行型专家禁购买 / 媒体 / 语音 / 跨 bot 调度。
- 全部固定 `deepseek-flash`；调度/产品/UI 为 `max` effort，其余 `high`。
- 协作链路实测：调度用 `message_bot` 派单 → Memo 用 `kb_search` + `kb_get` 回路径 → 异步回传。

### 执行环境：docker → desktop

- `SANDBOX_PROVIDER` 从 `docker` 改为 `desktop`，清理卡在 `booting` 的 team computer 行后，专家 bot 的 `shell` 在本机成功执行。
- `pnpm sandbox:build` 曾因 Docker Hub / deb.debian.org 网络与 apt 代理不稳失败，`rakazo/computer:local` 镜像至今未生成——**不再是阻塞项**（已切 desktop）。

### 展示层调整

- 主 bot 流式 reasoning：`thinking_delta` → `AgentRuntimeEvent.reasoning` → `run-turn` 每 250ms 批量 + `redactSecrets` → `thread.progress {reasoning}` → web `ReasoningBlock`「思考」。仅 live 展示，不写进最终 durable 消息。
- subagent 进度：捕获嵌套 agent 的 `thinking_delta`，最新 2000 字写进 `progress`，`SubagentCard` 折叠显示一行。commit `7cfc54b`。
- 布局：全屏内容列 60rem → 80rem、气泡 39rem → 60rem、composer 同步；聊天 transcript 改 opencode 风格（assistant 去气泡平铺、user 保持右对齐 `bg-muted`、reasoning 改左侧竖线）。

## 待办 / 未决

- [ ] 主 bot 流式 reasoning + `SANDBOX_PROVIDER=desktop` 那批改动（`pi-runtime.ts`、`run-turn.ts`、`core/events.ts`、`message-view.tsx`、`thread-target.ts`）**尚未 commit**。
- [ ] `feat/colna-memo-and-skill-folder` 分支的 MR 未创建（已 push 到 origin）。
- [ ] 技能文件直写 DB 用的临时 `pg` 脚本未入库（设计上本该走 `agentSkills.create`，缺批量补文件的 API）。

## 关键命令

```bash
git clone --depth 1 https://github.com/colna/rakazo.git /Users/colna/WORK/rakazo
RAKAZO_LOCAL_NO_AUTH=true SIGNUPS_ENABLED=false pnpm --filter @rakazo/api dev
pnpm --filter @rakazo/desktop dev
pnpm --filter @rakazo/{api,adapters,adapter-kit,core,web} check
```

## 来源

2026-11-02 Daily 11:30 / 16:35 / 18:35 / 19:38 / 19:44 / 19:47 / 20:11 / 20:36 / 20:58 / 21:36 十条工作日志，均来自当日实际交付与验证记录。
