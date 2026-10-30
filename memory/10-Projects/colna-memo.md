---
title: colna-memo
date: 2026-06-25
tags: project, rust, knowledge-base, zvec, embedding, mcp, para
---

# colna-memo

跨设备个人知识库:**Git 存 Markdown 为唯一真源 + zvec 本地语义索引**。本目录的 `memory/` 既是这个项目的产物,也是知识库本体;此笔记记录的是**项目/工具本身**(与知识库内容区分)。

## 基本信息

- 仓库:`git@github.com:colna/colna-memo.git`(使用本机 SSH 配置)
- 路径:`/Users/colna/WORK/colna-memo`
- 当前分支:`main`
- 语言:Rust

## 架构

- **Git 为唯一真源**:所有内容是 `memory/` 下 Markdown,走 git 跨设备同步。
- **zvec 为派生索引**:每台设备本地建语义向量索引,`.colna/` 不入 git,可随时 `colna index` 重建。
- **state v2 做完整性证明**:记录文件 hash、每文件 chunk IDs 和总数;增量前核对 zvec doc_count / index completeness,不可信即全量恢复。
- **有界索引**:embedding 与写入按 16 chunks 串行批处理,写完执行 zvec optimize。
- 绕开嵌入式向量库"单进程写"限制,无多设备写同一 DB 冲突。

## 技术栈

- [zvec](https://github.com/zvec-ai/zvec-rust) v0.5.0 — 进程内向量库(git 依赖)
- [fastembed](https://github.com/Anush008/fastembed-rs) — 本地 embedding,模型 `multilingual-e5-small`(384 维,中英多语言,离线)
- 检索:向量 + FTS 混合(RRF 融合)
- MCP server(stdio)供 Claude 调用 `kb_search` / `kb_get`

## CLI(`./colna` 包装脚本,处理 zvec 动态库 rpath)

| 命令 | 作用 | 推远端 |
|------|------|--------|
| `index` | 扫描 memory/ 增量重建索引 | 否 |
| `search` | 语义 / 混合检索 | 否 |
| `mcp` | 以 MCP server(stdio)运行 | 否 |
| `add` | 新建笔记到 `00-Inbox/` + 自动 reindex | 否 |
| `sync` | commit memory → pull --rebase --autostash → reindex → push | **是** |

## 关键约定

- 知识库内容采用 PARA + Inbox + Daily,书写约定见 `memory/CONVENTIONS.md`。
- `_` 开头目录(`_templates/` `_attachments/`)不入索引。
- 构建:`cargo build`(首次下载 zvec 预编译库);跑二进制需 `DYLD_LIBRARY_PATH`,用 `./colna` 自动处理。
- **改完 memory/ 即 `colna sync` 同步到远端**(工作区规则授权的自动动作)。

## 2026-07-31 索引 / 同步修复

- `fastembed 4.9.1` 默认把最多 256 条分成多个 Rayon 并发 ONNX batch,每个 Session 又使用全部 CPU 线程;1355 chunks 曾达到约 29.5 GB footprint / 28.6 GB swap。
- 应用层改为每批 16 chunks 串行,并显式传 `Some(batch.len())`;真实全库 108 files / 1355 chunks 用时 107.35 秒,最大 RSS 2.04 GB,swap 0。
- 全量中断先清 state;成功前校验写入计数、zvec doc_count、schema 中 `text` / `embedding` 索引存在性和 vector completeness,防止部分 index 被误报为完成。zvec v0.5.0 的 stats 只列向量索引,FTS 存在性需通过 schema 单独检查。
- zvec v0.5.0 的 `delete_by_filter` 在本 schema 的优化后 collection 上成功返回但不删文档;state v2 改存 chunk IDs,增量按主键幂等删除。
- `sync` 在 pull 前后都检查 Git 中间态 / unmerged;autostash 恢复冲突即停止,不再 reindex / push。

## 2026-10-30 竞品对比:OpenMuse / Rakazo

> 依据两个仓库的 README 与 docs(未逐行读源码);colna 侧依据本仓库 README + `src/`。

### 一句话定位

| 项目 | 定位 | 形态 |
|------|------|------|
| [OpenMuse](https://github.com/CopilotKit/openmuse) | 单用户「个人 agent 工作台」:浏览器 + 终端 + 文件 + 可中断/恢复的持久任务 | Expo/RN(iOS/Android/Web)+ Hono/CopilotKit 服务端 + Playwright worker + 可选 Docker computer |
| [Rakazo](https://github.com/elie222/rakazo) | 可自托管的「AI 队友平台」:多 bot、记忆、routine、技能、团队空间 | Web + Electron + Expo 移动端;Hono/oRPC + Postgres/Prisma + Graphile Worker |
| colna-memo | 本地优先的「长期记忆底座」:Git 存 Markdown,本地混合检索,给 agent 读 | Rust CLI + stdio MCP;无 UI、无 agent runtime |

### 能力速览

**OpenMuse**:CopilotKit 原生聊天(AG-UI 流式、富线程/侧聊/重放,线程持久化必须用 CopilotKit Intelligence);持久任务引擎(计划、检查点、暂停/恢复/取消/重试、审批、receipt、SQL lease);持久 Chromium(可接管、截图、PDF 下载)+ 可选隔离 Linux 容器(非 root、断网、30s 命令上限、持久 `/workspace`、文件/PDF);Gmail/Calendar OAuth(草稿、附件、日程 CRUD + 审核);Ideas(带来源证据)、Goals(公网页变更/文本/价格阈值监控 + 去重告警)、Documents(PDF 表单填写)、Finance(CSV 支出汇总)、可编辑个人记忆;单 owner access key,非多租户;alpha。

**Rakazo**:持久 bot(对话、记忆、routine、历史);记忆 = 每 bot `MEMORY.md` + 可选 Supermemory/Serenity/Honcho 语义记忆,支持 isolated/shared scope,记忆不跨 space;routine(定时/cron、文件或命令变更监控、webhook、GitHub 事件、Slack);skills(技能库、GitHub 导入、任务后保存、录屏教学生成草稿 skill);多 bot 群聊、bot 间消息、子代理、bot 创建/管理 bot(带审批);Team/Private computer(浏览器、终端、文件、图形桌面;Docker/E2B/Daytona/CreateOS/Boat/Railway/本机);语音、Composio/Pipedream/MCP/OpenAPI 集成;spaces 隔离 + 审批模式(Ask/Auto/Never)+ RLS;beta,Apache 2.0。

### 共同点

1. **本地优先 / 数据自持**:三者都强调自托管、BYO model、避免 SaaS 锁定。
2. **记忆是核心资产**:OpenMuse 有可编辑记忆 + Rich Threads,Rakazo 有 `MEMORY.md` + 语义 provider,colna 直接以记忆为本体。
3. **给 agent 提供工具面**:colna 用 MCP,另外两家用 tool calling;方向一致。
4. **文件/纯文本是通用接口**:agent 最终都在操作文件;colna 的真源就是文件。
5. **证据/可追溯**:OpenMuse 的 receipt、Rakazo 的 routine/tool activity、colna 的 Daily/工作日志/troubleshooting 解决的是同一类问题。
6. **单用户/小团队优先**:OpenMuse 单 owner、Rakazo 个人 + 小团队、colna 纯个人。

### 关键差异

| 维度 | colna-memo | OpenMuse | Rakazo |
|------|------------|----------|--------|
| 核心 | 记忆/检索 | 单 agent 执行工作台 | 多 bot 队友平台 |
| 运行时 | 无 | 有(task worker + browser worker) | 有(worker + 多 sandbox provider) |
| 执行环境 | 无 | Chromium + Docker Linux | 浏览器/终端/桌面/云沙箱 |
| UI | CLI + MCP | iOS/Android/Web 聊天 | Web/桌面/移动/语音 |
| 存储 | Markdown + Git | PGlite/Postgres + Intelligence | Postgres/Prisma |
| 检索 | 本地 E5 向量 + FTS + RRF | 未强调本地向量检索 | `MEMORY.md` + 外部记忆服务 |
| 跨设备 | Git 同步 | 服务端集中 | 服务端集中 |
| 自动化 | 无 | 持久任务 + 目标监控 | routine + webhook + Git/Slack 触发 |
| 外部集成 | 无 | Gmail/Calendar/浏览器/搜索 | Composio/Pipedream/MCP/OpenAPI/语音 |
| 治理 | 约定 + 工作日志 | 审批 + receipt | 审批 + spaces + fleet 管控 |
| 多用户 | 否 | 单 owner | 账号/空间/RLS |
| 成熟度 | P0–P3 已落地 | alpha | beta |

### colna 可升级的点

**P0(直接补短板,1–3 天/项)**

1. **MCP 写接口 + 审核**:`kb_put` / `kb_append` / `kb_update` / `kb_archive`;默认只写 `00-Inbox/`,frontmatter 校验,`expected_hash` 乐观锁,dry-run diff + 确认,永不硬删。现在 agent 只能读,记忆闭环是断的。
2. **检索过滤/排序**:`--tag`、`--path`、`--since/--until`、`--type`;recency boost、MMR 去重、返回 heading 路径 + 摘要 + 分数构成;`kb_search` 同步支持。PARA 结构不落到检索过滤上等于没用。
3. **索引新鲜度**:`search --ensure-index`(按 state hash 快速校验)或 `colna watch`(FSEvents/notify + 锁);现在手动改文件后搜索会静默用旧索引。
4. **`colna clip <url>`**:defuddle/Jina 抽正文 → Inbox,frontmatter 记 `source_url` / `fetched_at` / `content_hash`;把现有手工剪藏流程产品化。
5. **`colna context "<task>"`**:输出紧凑上下文包(top notes + 当天 Daily + 项目约定 + troubleshooting + 相关链接),任何 agent 可直接注入;对应 OpenMuse 的 personal context,但不需要 UI。

**P1(1–2 周/项)**

6. **记忆生命周期**:`colna digest` / `colna consolidate --dry-run`:Daily → 项目/troubleshooting/待办,按内容 hash + 语义相似去重,过期归档,"open work" 清单;可选 LLM,保留确定性 fallback。
7. **笔记图谱**:解析相对链接/wikilink,建 backlinks;`colna related <note>`、`colna graph --json`;检索时给被链接笔记加权。
8. **切块/元数据升级**:按标题层级 + token 预算切块,文档摘要向量,tags/date 变成可过滤字段,代码块单独处理;需要索引 schema 版本迁移。
9. **检索评测**:`colna eval`(query → 期望路径,recall@k / MRR),作为换模型、改切块、加 rerank 的回归门槛;没有评测就不要调检索。
10. **附件/文档摄取**:PDF 文本、HTML、图片 OCR、会议转写,保留 provenance;这是 OpenMuse/Rakazo 的强项、colna 的空白。
11. **重排/查询扩展**:可选 cross-encoder 或 LLM query rewrite,多 query RRF;先有评测再上。

**P2(按需/谨慎)**

12. **routine / watch**:`colna digest` 定时;文件或 Git 变更触发 reindex + 摘要;URL 文本/价格监控写回 Daily(借鉴 OpenMuse Goals)。
13. **任务 receipt**:定义 task note schema(目标、计划、检查点、证据、结果)+ MCP 工具,借鉴 OpenMuse 的持久任务;只做记录,不做 worker。
14. **scope / 多 vault**:`--scope work|personal` 或多 root,跨 scope 需显式开启;对应 Rakazo spaces。
15. **加密/脱敏**:age/SOPS 加密敏感笔记,`.colnaignore`,入库前 secret 扫描。
16. **技能索引**:把 `~/.agents/skills/*/SKILL.md` 作为独立 collection 索引,提供 `kb_skill_search`;对应 Rakazo skills。
17. **最小本地 UI**:只读 `colna serve`(搜索/backlinks/图谱),不做聊天。

### 可以结合的点

- **A. colna 做 OpenMuse 的记忆层**:OpenMuse 保留 UI、任务引擎、浏览器/终端;把 personal context 和任务 receipt 落到 colna(MCP/HTTP),所有长期知识进 Git。执行与记忆分离,边界最干净。
- **B. colna 做 Rakazo 的 memory provider**:Rakazo 已有 Supermemory/Serenity/Honcho 的 provider 抽象;实现一个 colna adapter,把 space 映射到 vault/scope,bot 就能用本地混合检索,不依赖 SaaS。
- **C. 用它们做前端、colna 做后端**:聊天/执行/语音用 OpenMuse 或 Rakazo,产出统一 `colna capture` 成 Markdown;人用 Obsidian/Git 浏览。
- **D. 借 routine 模型**:`colna digest` 定时;文件/Git 变更触发 reindex + 摘要;webhook 通知飞书。
- **E. 借 task receipt 模型**:把「计划→执行→证据→结果」写成结构化 note,既能续跑也能审计。
- **F. 借 skills/teach**:把演示出来的流程沉淀成 note/skill,再索引成可检索的过程性记忆。
- **G. 借审批模型**:agent 写库必须 diff + 审批(规范模式),防止记忆被污染。
- **H. 借 spaces**:工作/个人隔离,按项目 scope 检索。

### 不建议做

- 不做聊天 UI / agent runtime / 多 bot / 语音 / 桌面;这是它们的护城河,也是巨大维护成本。
- 不迁 Postgres/Prisma,不引入 CopilotKit Intelligence 这类外部必需依赖;Git + Markdown 是 colna 的差异化。
- 不让 agent 无审核写库;不盲目索引附件和密钥。
- 不追 Gmail/Calendar/Composio 这类连接器;它们是执行层能力,不是记忆层。

### 结论

三者不是替代关系:OpenMuse 是「单 agent 工作台」,Rakazo 是「多 agent 队友 OS」,colna 是「记忆底座」。colna 最值得走的路不是变成第三个 agent 产品,而是把「捕获 → 检索 → 执行 → 记录 → 巩固」里的记忆环节做深,并通过 MCP 成为它们缺的那层本地、可同步、可审计的长期记忆。

## 2026-10-30 P0 落地:写入闭环 / 检索过滤 / 新鲜度 / context / clip

### 已实现

- **MCP 写接口**:`kb_put` / `kb_append` / `kb_update` / `kb_archive`。
  - 默认 `confirm=false` 只返回 diff/预览;`confirm=true` 才落盘。
  - 新建默认 `00-Inbox/`,路径拒绝绝对路径 / `..` / `_` 前缀目录。
  - `append` / `update` 必须带 `expected_hash`(sha256 乐观锁);`archive` 只移动到 `40-Archive/`,永不硬删。
  - 写入后自动增量 reindex;MCP 的 stdout 保持纯 JSON,进度走 stderr。
- **检索过滤 / 排序**:`--tag`(AND)、`--path`、`--since` / `--until`、`--type`;
  时效加权 `--recency-weight`(默认 0.15)、MMR 去重(`--no-mmr` 关闭)、
  结果输出 `vec` / `fts` / `recency` / `mmr` 分数构成。
- **索引新鲜度**:`search` 默认先做增量校验(`--no-ensure-index` 跳过);
  `.colna/index.lock` 进程锁,按 PID 判断存活,长索引不会被误删锁。
- **`colna context "<task>"`**:书写约定 + 今日重点 + 最近工作日志 + 相关笔记 + 排错经验,`--max-chars` 控制预算。
- **`colna clip <url>`**:`COLNA_CLIP_CMD` → 本地 defuddle → r.jina.ai;落 `00-Inbox/`,
  front-matter 记 `source_url` / `fetched_at` / `content_hash`。
- **索引 schema v4**:新增 `breadcrumb` 字段;state 记录 `model` / `chunker_version`,
  模型或切块规则变更自动全量重建。

### 改动文件

- 新增:`src/indexer.rs`(索引抽离)、`src/search.rs`(过滤/时效/MMR)、`src/write.rs`(写入 + 预览)、
  `src/context.rs`、`src/clip.rs`、`src/util.rs`。
- 修改:`src/main.rs`(CLI 新命令/参数)、`src/mcp.rs`(7 个工具)、`src/store.rs`(breadcrumb + 原始分)、
  `src/chunker.rs`(标题面包屑)、`src/state.rs`(v4 + 模型/切块版本)、`src/gitsync.rs`(移除重复写入逻辑)、
  `src/embedder.rs`(模型 ID 常量)、`README.md`。
- 未提交代码(规范模式);`memory/` 走 `colna sync`。

### 验证

- `cargo test`:42 passed;`cargo clippy --all-targets -- -D warnings` 通过。
- 临时知识库验证:MCP `kb_put` 预览/确认、`kb_append`、`kb_update`、`kb_archive`、`kb_context` 全链路通过;
  `kb_put` 的 diff 用 `diff -u` 输出,确认前不落盘。
- 真实库已因 schema v4 全量重建:333 个文件 / 4586 chunks,state v4 + e5-base + chunker 2;
  `colna search` / `context` / MCP `kb_search` / `kb_get` / `kb_context` / `kb_put` 预览均通过。
- 真实库 `kb_put` 预览路径 `00-Inbox/P0-Smoke-Test.md` 确认未落盘。

## 2026-10-30 P1/P2 落地:整理 / 图谱 / 评测 / 摄取 / 任务 / 安全 / 技能

### P1

- **Daily 整理**:`colna digest`(今日重点 / 工作日志摘要 / 可沉淀经验 / 链接;`--semantic` 标相似笔记)、
  `colna consolidate --apply`(未完成项转今日 Daily + 幂等写 Digest)、`colna work [--sync]`(open work 清单)。
- **笔记图谱**:`colna graph` / `colna related <path>`;解析相对 Markdown 链接与 `[[wikilink]]`,
  检索时按入链数加权(`--link-boost`,默认 0.05)。
- **切块升级**:schema v5 + chunker 3;长章节按 ~1200 字符切分且不切断代码块;每篇文档增加摘要块
  (`kind=summary`);`--no-summary` 可关闭摘要召回。
- **检索评测**:`colna eval eval/smoke.json` 输出 recall@k / MRR;固定 6 条 smoke 用例。
- **附件摄取**:`colna ingest <path>`;PDF(`pdftotext`)、HTML(`textutil`/`pandoc`)、图片(`tesseract`)、
  音视频(`COLNA_WHISPER_CMD`);frontmatter 保留 `source_path/source_type/extracted_at/content_hash`。
- **查询扩展 / 重排**:`--expand` 读 `COLNA_QUERY_EXPAND_CMD`;`--rerank` 用 fastembed BGE cross-encoder
  (首次下载,默认关闭)。

### P2

- **Routine / watch**:`colna watch [--interval] [--once]` 轮询增量索引;
  `colna watch-url <url> [--contains/--price]` 把变化写回 Daily `## 观察`,状态在 `.colna/watches.json`。
- **任务 receipt**:`colna task start/checkpoint/finish/list`;frontmatter `status/project/goal`,
  正文 `Goal/Plan/Checkpoints/Result/Evidence`;MCP 对应 `kb_task_*`。
- **Scope / 多 vault**:`.colna/scopes.json` 把 scope 映射到路径前缀;`--scope` / MCP `scope`。
  多 vault 仍用 `--root`。
- **安全**:`.colnaignore` 排除索引;`colna secrets` 扫描私钥/token/凭据赋值,`--fix` 脱敏;
  `colna encrypt/decrypt` 包装 `age`(本机未安装 age,未做真机验证)。
- **技能索引**:`colna skills index` / `colna skills search`;独立 `.colna/skills.zvec`,不混入 memory/ 主库;
  MCP `kb_skill_search`。

### 验证

- `cargo test`:60 passed / 1 ignored(大集合 zvec finalize 回归);`cargo clippy --all-targets -- -D warnings` 通过。
- 临时库端到端:digest / consolidate --apply / work / graph / related / eval / scope / secrets --fix /
  ingest(txt/html) / task / watch-url / skills index+search / MCP `kb_task_*` 全链路通过。
- 真实库:state v5 / e5-base / chunker 3,330 files / 5665 chunks;`eval/smoke.json` recall@k 1.000 / MRR 0.639;
  `skills index` 81 个技能;MCP `kb_search` / `kb_context` / `kb_get` 读工具通过。
- 索引加固:把 `.colna/index.lock` 单文件改成 `.colna/index.lock.d/` 目录(按 PID 判活,`rm -f index.lock` 不再能误删);
  此前并行 `colna sync` 删除锁文件后与长索引并发,把 `scalar.0.ipc` 写成 0,已复现并修复。
- 未验证:`--rerank`(需下载 BGE reranker 模型)、age 加密(本机无 age)、whisper 转写(本机无 whisper)。
