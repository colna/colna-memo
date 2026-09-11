---
title: Cline 全局规则与 skills 的落盘位置
date: 2026-09-11
tags: troubleshooting, cline, agent-skills, rules
---

# Cline 全局规则与 skills 的落盘位置

**问题**：想让 Cline（VS Code 扩展 `saoudrizwan.claude-dev`）遵守某套个人规则、或装上某些 skill，但不知道文件该放哪；把规则写进 `~/.claude/CLAUDE.md` 却完全不生效。

**结论**（实测 Cline **v4.1.17** 的 `next/dist/extension.js`，2026-09-11）

## 一、规则：Cline **不读 `CLAUDE.md`**

bundle 里认的字面量只有这五个：`AGENTS.md`、`.clinerules`、`.cline/rules`、`.cursorrules`、`.windsurfrules`（另有 `.clinerules/{hooks,skills,workflows}` 子目录）。
`CLAUDE.md` 是 Claude Code 的入口（`@path` 导入语法也是它专有），**Cline 不解析** —— 所以写在 `~/.claude/CLAUDE.md` 里的「禁止自动 commit/push」之类，对 Cline 零约束。

| 作用域 | 位置 | 说明 |
| --- | --- | --- |
| 全局（所有工作区） | `~/Documents/Cline/Rules/*.md` | 就是它的全局规则目录，UI 里叫 **Global Rules** |
| 全局（新路径） | `~/.cline/rules` | `g4() = $CLINE_DIR \|\| ~/.cline`，两个都读 |
| 工作区 | `AGENTS.md` / `.clinerules`（文件或目录）/ `.cline/rules` / `.cursorrules` / `.windsurfrules` | UI 里叫 Workspace Rules，**没有新建入口**，得自己在仓库里建 |

**手工放文件即可生效**：刷新规则时走 `MF(dir, toggles)` —— 目录里出现但不在 toggles 里的路径直接 `a[path] = true`，也就是**默认启用**；单条开关存在 `globalClineRulesToggles`（key = 文件绝对路径）。UI 结构：`Rules` 页 = `Enterprise Rules`（远程只读）+ `Global Rules`（可新建）+ `Workspace Rules`（不可新建）。

> ⚠️ 与「不要修改 `~/Documents`、`~/Desktop`、`~/Downloads`」这条通用规则**字面冲突**：必须补一条例外，否则机器人自己不能维护它的全局规则。

## 二、skills：真源 `~/.agents/skills/`，其余目录用软链

Cline 读的全局 skills 目录：`~/.cline/skills` 与 `~/.agents/skills`；工作区读 `.clinerules/skills`、`.cline/skills`、`.claude/skills`、`.agents/skills`。
（`~/.claude/skills`、`~/.cursor/skills`、`~/.codex/skills` 是**别的 agent** 的目录，Cline 不读它们。）

**做法**：`~/.agents/skills/` 当唯一真源，`~/.claude|.cursor|.codex|.cline/skills` 全放**相对软链** `../../.agents/skills/<name>`。

- 增删改只动真源一处，四个 agent 同步生效；
- **软链不是实体副本** —— 放实体副本必然与真源脱节（本次就发现 `byteplus-query` 两边各有一份、内容已分叉）；
- 校验要点：每个 skill 目录必须有 `SKILL.md`，frontmatter 的 `name` 必须等于目录名，否则可能不被加载；
- 幂等同步脚本：`/tmp/sync-skills.py`（**临时目录，重启即失**，要长期用必须挪走）。

## 三、坑：换了 MCP 的 key，旧进程还在用旧的

改 `cline_mcp_settings.json` 里的 key 后，旧 MCP 进程不会立刻消失；但 Cline **监听该 settings 文件**，会自动拉起带新 key 的进程（实测：改完不到 1 分钟新进程就起来了，`ps` 里能同时看到新旧两组 `figma-developer-mcp`）。

**校验新 key 别只靠 MCP 工具**，直接打上游更可靠：

```bash
curl -sS -H 'X-Figma-Token: <token>' https://api.figma.com/v1/me   # → 200 + 账号名
```

## 四、教训

- **「约定写在文档里」≠「机制生效」**（同类前科：`CLAUDE.md` 说靠 `includeIf` 统一 git 身份，实际没有）。要让某个 agent 守规则，先确认**它读哪个文件**，再确认**文件真被读到了**（新会话里让它复述规则，或看设置面板的开关）。
- 全局规则会作用于**所有**工作区：写之前先想清楚会压掉哪些仓库自己的约定；冲突时以「先说明冲突、由用户裁决」为原则，别静默取一方。
- 把一套规则从一个工作区搬到「全局」时，**路径要做适配**（本次：`/Users/user/Dev2/zhangzheng/` → `/Users/colna/WORK/`、`colna` 不在 PATH 要写绝对路径、去掉 `github-colna` 别名与 app id 硬编码）。原样照抄一份跨机器不可用的规则，比没有规则更危险。
