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

**2026-09-11 补：token 会过期，过期是全局的。** Figma 的 PAT 有有效期，过期后**所有**文件的
`get_figma_data` 都回 `403 {"err":"Token expired"}`（不是某一个文件没权限，别往分享设置上想）。
这时只有换 key：重新生成 PAT，替换 `cline_mcp_settings.json` 里
`github.com/GLips/Figma-Context-MCP` 这个 server 的 `args` 里 `--figma-api-key` 后面那个值：

```
~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json
```

改完 Cline 会自动重载（见上）。`~/.claude.json`、`~/.cursor/mcp.json` 里各有一份同名 key
（另外两个 agent 用），要一起换就一起换。**token 不要贴进聊天**，改完用上面那条 curl 验。

## 四、教训

- **「约定写在文档里」≠「机制生效」**（同类前科：`CLAUDE.md` 说靠 `includeIf` 统一 git 身份，实际没有）。要让某个 agent 守规则，先确认**它读哪个文件**，再确认**文件真被读到了**（新会话里让它复述规则，或看设置面板的开关）。
- 全局规则会作用于**所有**工作区：写之前先想清楚会压掉哪些仓库自己的约定；冲突时以「先说明冲突、由用户裁决」为原则，别静默取一方。
- 把一套规则从一个工作区搬到「全局」时，**路径要做适配**（本次：`/Users/user/Dev2/zhangzheng/` → `/Users/colna/WORK/`、`colna` 不在 PATH 要写绝对路径、去掉 `github-colna` 别名与 app id 硬编码）。原样照抄一份跨机器不可用的规则，比没有规则更危险。

## 五、Claude Code 侧的全局规则：`~/.claude/CLAUDE.md`（2026-09-14 补）

上文的「Cline 不读 `CLAUDE.md`」只对 Cline 成立 —— **Claude Code 读的就是它**，而且是**用户级全局**（所有工作区都会加载），所以同一份个人规则要落两处（Cline 那边见第一节）。

2026-09-14 把工作区真源 `metabot-workspace/AGENTS.md` 适配成全局版写进 `~/.claude/CLAUDE.md`（355 行，旧文件备份 `CLAUDE.md.bak-2026-09-14`）。**适配点**（照本文「四、教训」做的）：

1. **skills 路径**：工作区版写 `.claude/skills/`；全局语境下真源是 `~/.agents/skills/`（73 个），`~/.claude|.cursor|.codex|.cline/skills` 均是指向它的**相对软链**。全局版必须写真源，否则会诱导后来者往软链目录放实体副本（第二节的 `byteplus-query` 分叉就是这么来的）。
2. **不在 PATH 的命令**：`colna` 不在 PATH，统一写成 `cd /Users/colna/WORK/colna-memo && ./colna ...`。
3. **作用域切分**：只对某一类会话有意义的规则（MetaBot 的 `OUTPUTS_DIR`、`/reset`、`/stop`、飞书上线申请模板）收进文末「附」一节并注明「非飞书会话可忽略」，别让全局规则里混着一半用不上的东西。
4. **补全差异**：工作区只有 43 个 skill，全局有 73 个 —— 全局版要补上 `lark-*` 一族、`byteplus-query`、`umi`、`html`、`rust-best-practices`、`obsidian-*`、`find-skills`、`code` 的触发条件，否则这些 skill 等于没有触发指引。
5. **冲突声明**：全局规则会压到所有仓库，所以要在文件头部写明「工作区自带 `AGENTS.md` / `CLAUDE.md` 冲突时以工作区为准，且先说明冲突由用户裁决」。
6. **通用规则的例外**：工作区规则说「不要改 `~/Documents`/`Downloads`」，但 Cline 的全局规则目录恰恰在 `~/Documents/Cline/Rules/` —— 搬到全局时必须显式写例外，否则规则自相矛盾。

**待办 / 未证实**：`claude -p "复述规则"` 的生效验证被中断，尚未确认新会话真读到了这份文件（这正是「约定写在文档里 ≠ 机制生效」那条教训的检查动作，要补）。另发现 `metabot-workspace/CLAUDE.md`（旧 Obsidian 版，指向 `COLNA's wiki`）与 `~/.codex/AGENTS.md`（0 字节）两处漂移，未处理。
