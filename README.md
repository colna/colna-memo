# colna-memo

跨设备的个人知识库,用来记录 Claude 的上下文与个人笔记。

## 架构

```
        ┌──────────── Git 远端(唯一真源) ────────────┐
        │   memory/  (Markdown: 笔记 / 对话 / 文档)      │
        └──────────────────────────────────────────────┘
              ↑ git push/pull(跨设备同步靠这个)
   设备A 本地                          设备B 本地
   ├ memory/        (md 真源)         ├ memory/
   ├ .colna/index.zvec ← 本地派生索引  ├ .colna/index.zvec ← git pull 后重建
   └ colna (CLI)                       └ colna (CLI)
```

- **Git 为唯一真源**:所有内容是 `memory/` 下的 Markdown,走 git 跨设备同步。
- **zvec 为派生索引**:每台设备本地用 [zvec](https://github.com/zvec-ai/zvec-rust) 建语义向量索引,
  索引文件 `.colna/` **不入 git**,可随时由 `colna index` 重建。
- 这样绕开了嵌入式向量库"单进程写"的限制,不存在多设备写同一 DB 的冲突。

## 技术栈

- Rust
- [zvec](https://github.com/zvec-ai/zvec-rust) v0.5.0 — 进程内向量数据库(git 依赖)
- [fastembed](https://github.com/Anush008/fastembed-rs) — 本地 embedding,模型 `multilingual-e5-base`(768 维,中英多语言,离线、数据不外传)

## 构建

```bash
cargo build            # 首次会下载 zvec 预编译库
```

## 使用

由于 zvec 预编译动态库的 rpath 限制,直接跑二进制需要设 `DYLD_LIBRARY_PATH`。
仓库提供了 `./colna` 包装脚本自动处理:

```bash
./colna index                      # 增量更新本地索引(只重嵌入变化的文件)
./colna index --full               # 强制全量重建

./colna search "怎么跨设备同步"      # 混合检索(默认先校验索引新鲜度)
./colna search "xxx" --topk 8
./colna search "xxx" --semantic-only
./colna search "xxx" --tag rust --tag kb          # 标签 AND 过滤
./colna search "xxx" --path 10-Projects/colna-memo
./colna search "xxx" --since 2026-01-01 --until 2026-12-31
./colna search "xxx" --type project               # inbox/project/area/resource/archive/daily
./colna search "xxx" --no-ensure-index            # 跳过索引校验(更快,可能读到旧索引)
./colna search "xxx" --no-mmr                     # 关闭 MMR 去重

./colna search "xxx" --expand --rerank             # 查询扩展 + cross-encoder 重排
./colna search "xxx" --scope work                  # 读 .colna/scopes.json 的路径前缀
./colna related 10-Projects/colna-memo.md          # backlinks + 语义邻居
./colna graph --json                               # 图谱
./colna eval eval/smoke.json --topk 5              # recall@k / MRR 评测

./colna context "给 colna 加 MCP 写接口" --topk 8 # 生成可注入 agent 的上下文包
./colna clip "https://example.com/post" --tags clip # 网页正文 → 00-Inbox/
./colna ingest ~/Downloads/report.pdf              # PDF/HTML/图片/音视频 → Markdown

./colna digest --date 2026-10-29 --semantic        # Daily 整理建议
./colna consolidate --apply                        # 未完成项转入今日 Daily + 写 digest
./colna work --sync                                # 生成 00-Inbox/open-work.md
./colna task start "给 colna 加 P1" --project colna-memo
./colna task list --status open
./colna watch --once --digest                      # 索引变化检查 + 当天 Digest(可配 cron)
./colna watch-url "https://example.com" --contains "Example Domain"
./colna secrets                                    # 敏感信息扫描(默认只报告)
./colna skills index && ./colna skills search "MCP"

./colna add "标题" -b "正文" -t "tag1,tag2"  # 新建笔记到 memory/00-Inbox/ 并自动增量索引
echo "正文" | ./colna add "标题"            # 正文留空则从 stdin 读取
./colna sync                               # 跨设备同步:commit → pull --rebase → reindex → push
./colna sync -m "今天的笔记"                # 指定提交说明
```

### 索引新鲜度

`colna search` 默认先做一次增量校验:按文件 sha256 对比 `.colna/state.json`,
有变化才重新切块 / 嵌入 / 写索引。手动改完 Markdown 直接搜索不会静默用旧索引。
追求更低延迟时用 `--no-ensure-index`。

索引写入使用 `.colna/index.lock.d/` 目录锁(内含 PID);进程存活时不会被误删,异常退出后按 PID / 10 分钟 stale 清理。

### 增量索引

`colna index` 默认增量:用 `.colna/state.json` 记录格式版本、embedding 模型、切块版本、
文件 sha256、chunk 主键和总数,只对**新增 / 变更 / 删除**的文件做处理,首次或 `--full` 才全量重建。
embedding 与 zvec 写入按固定小批次串行执行,避免全量索引时多个 ONNX batch 并发占满内存。
写入后执行 zvec optimize,并校验 doc_count、必需 schema 索引与向量索引 completeness;旧版 / 损坏 /
缺失 state、索引打不开或数量 / 必需索引不一致时都自动执行干净的全量重建。增量删除按 state 中的 chunk 主键执行,
避免依赖 zvec v0.5.0 在当前 schema 下不可靠的 `delete_by_filter`。

v5 起索引包含 `breadcrumb`(标题路径)、`kind`(`section` / `summary`)字段,
并记录 `model` / `chunker_version`;模型或切块规则变更会自动触发一次全量重建。
长章节按 ~1200 字符切分(不切断代码块),每篇文档额外生成一个摘要块用于文档级召回。

### 混合检索与排序

`colna search` 默认混合检索:同时跑

- **向量召回**:HNSW + cosine,语义近邻;
- **FTS 召回**:`text` 字段的全文索引,关键词精确命中;

再用 RRF(Reciprocal Rank Fusion)融合两路排名。之后:

- **过滤**:标签(AND)、路径前缀、日期区间;
- **时效加权**:`--recency-weight`(默认 0.15,相对候选池最高分;0 关闭);
- **MMR 去重**:减少同一笔记/近重复段落的堆叠(`--no-mmr` 关闭);
- **入链加权**:按笔记图谱的入链数加权(`--link-boost`,默认 0.05;0 关闭);
- **分数构成**:输出 `vec`(cosine 距离)、`fts`、`recency`、`links`、`mmr` 明细。

中文 FTS 依赖分词器,召回为空时自动退化为纯向量,不影响结果。

### 查询扩展 / 重排

- `--expand`:读取 `COLNA_QUERY_EXPAND_CMD`(命令收到 query 作为 `$1`,每行一个变体),
  多 query 召回后用 RRF 融合;没有配置时自动使用原 query。
- `--rerank`:使用 fastembed 的 BGE cross-encoder 重排;首次运行会下载模型,默认关闭。
- 建议先跑 `colna eval` 建立基线,再开这两个开关。

### 图谱与相关笔记

- `colna graph`:输出所有 Markdown 的相对链接 / `[[wikilink]]` 边。
- `colna related <path>`:列出 inbound / outbound,并追加语义邻居。
- 检索时默认用入链数做小幅加权;图谱按需扫描,不落盘。

### 检索评测

`colna eval <queries.json> [--topk 5] [--json]` 计算 recall@k 与 MRR。JSON 格式:

```json
[
  {"query": "MCP 写接口", "expected": ["10-Projects/colna-memo.md"]},
  {"query": "索引锁", "expected": ["30-Resources/troubleshooting/xxx.md"], "tags": ["troubleshooting"]}
]
```

换 embedding、改切块、加 rerank / 过滤前,先用固定用例跑一遍基线。

### 上下文包

`colna context "<task>"` 输出一份 Markdown:

1. `memory/CONVENTIONS.md` 的书写约定;
2. 当天 `50-Daily/YYYY-MM-DD.md` 的「今日重点」与最近工作日志;
3. 检索到的相关笔记(标题路径 + 路径 + 日期/标签 + 摘要);
4. `30-Resources/troubleshooting/` 下的排错经验。

适合在启动新任务时直接注入任何 agent;`--max-chars` 控制预算(默认 12000)。

### Daily 整理 / open work

- `colna digest [--date YYYY-MM-DD] [--semantic]`:提取今日重点、工作日志摘要、可沉淀经验、相关链接;
  `--semantic` 会用向量相似度标出「可能已记录」的笔记。
- `colna consolidate [--date] [--apply]`:默认 dry-run;`--apply` 把前一天未完成项转入今日 Daily 并写 digest。
- `colna work [--json]` / `colna work --sync`:列出所有未完成项,或生成 `00-Inbox/open-work.md`。

### 网页剪藏

`colna clip <url>` 按以下顺序抽正文:

1. 环境变量 `COLNA_CLIP_CMD`(命令后自动追加 URL);
2. 本地 `defuddle parse <url> --md`;
3. `https://r.jina.ai/<url>`。

正文写入 `00-Inbox/`,front-matter 额外记录 `source_url` / `fetched_at` / `content_hash`。
使用 `r.jina.ai` 时 URL 会发给该服务;需要完全本地时安装 defuddle 或设置 `COLNA_CLIP_CMD`。

### 附件摄取

`colna ingest <path>` 按扩展名抽取文本:

- `.pdf`:`pdftotext -layout`;
- `.html`:`textutil`(macOS)或 `pandoc`;
- 图片:`tesseract`(优先 `chi_sim+eng`,失败回退 `eng`);
- 音视频:设置 `COLNA_WHISPER_CMD`(命令收到文件路径作为 `$1`,输出转写文本)。

结果写入 `00-Inbox/`,front-matter 记录 `source_path` / `source_type` / `extracted_at` / `content_hash`。

### MCP server(给 Claude 用)

`colna mcp` 以 MCP 的 stdio transport(JSON-RPC 2.0,逐行)运行,暴露:

**读工具**

- `kb_search(query, topk?, semantic_only?, tags?, path?, since?, until?, type?, ensure_index?, mmr?, recency_weight?, link_boost?, include_summary?, scope?, expand?, rerank?)` — 检索知识库;
- `kb_get(source_path)` — 取回某个 Markdown 文件全文(相对 `memory/`);
- `kb_context(task, topk?, max_chars?, scope?)` — 生成上下文包;
- `kb_skill_search(query, topk?)` — 检索技能索引(先 `colna skills index`);
- `kb_work_list()` — 列出所有未完成项。

**写工具**

- `kb_put(title, body, tags?, path?, confirm?)` — 新建笔记,默认落 `00-Inbox/`;
- `kb_append(path, body, heading?, expected_hash?, confirm?)` — 追加;
- `kb_update(path, content, expected_hash, confirm?)` — 整篇替换;
- `kb_archive(path, expected_hash?, confirm?)` — 移动到 `40-Archive/`,永不硬删。

写工具默认 `confirm=false` 只返回 diff/预览;真正落盘要再调一次并传 `confirm=true`。
`kb_append` / `kb_update` 必须带 `expected_hash`(当前文件 sha256),防止覆盖并发修改。
写入成功后自动增量重建索引。

**任务工具**

- `kb_task_start(goal, project?, plan?, confirm?)`;
- `kb_task_checkpoint(path, note, confirm?)`;
- `kb_task_finish(path, status?, outcome, evidence?, confirm?)`;
- `kb_task_list(status?)`。

在 Claude / 客户端的 MCP 配置中加入(用绝对路径,`./colna` 会自动设好 dylib 路径):

```json
{
  "mcpServers": {
    "colna-memo": {
      "command": "/绝对路径/colna-memo/colna",
      "args": ["--root", "/绝对路径/colna-memo", "mcp"]
    }
  }
}
```

### 任务 receipt

`colna task start "目标" [--project X] [--plan "..."]` 创建结构化任务笔记:
front-matter 记录 `status` / `project` / `goal`,正文分 `Goal` / `Plan` / `Checkpoints` / `Result` / `Evidence`。
`task checkpoint` 追加检查点,`task finish --status done|blocked` 更新状态并写结果,
`task list --status open` 列出任务。只做记录,不跑 worker。

### scope / 多 vault

- 多 vault:用 `--root` 指向不同知识库。
- 同一库内隔离:创建 `memory/.colna-scopes.json`(随库同步)或 `.colna/scopes.json`(本机覆盖),例如
  `{"scopes":{"work":"10-Projects/work","personal":"00-Inbox/personal"}}`;
  `search` / `context` / `kb_search` / `kb_context` 传 `--scope work` 或 MCP `scope`。
- 不传 scope 或传 `all` 表示全库;跨 scope 必须显式指定。

### 安全:ignore / secrets / age

- `.colnaignore`(仓库根目录):按路径前缀、后缀或简单通配符排除索引,例如 `00-Inbox/secret.md`。
- `colna secrets`:扫描私钥块、常见 token、`password/secret/token` 赋值;默认只报告,`--fix` 脱敏并重建索引。
- `colna encrypt <path> [--recipient <age key>]`:用 `age` 加密(需安装);`colna decrypt <path>` 解密到 stdout。
  加密后的 `.age` 不在 `.md` 索引范围内;明文文件建议加进 `.colnaignore` 或移出库。

### 技能索引

`colna skills index [--dir ~/.agents/skills]` 把 `SKILL.md` 建到独立 `.colna/skills.zvec`;
`colna skills search "..."` / MCP `kb_skill_search` 检索。技能不混入 memory/ 主库。

### 索引 watcher / URL 监控

- `colna watch [--interval 60] [--once] [--digest]`:轮询索引变化并增量更新;`--digest` 会在变化后生成当天 Digest。
- `colna watch-url <url> [--contains TEXT] [--price] [--note PATH]`:记录页面哈希 / 关键词 / 首个美元价格,
  变化时写回 Daily 的 `## 观察`;状态在 `.colna/watches.json`。

### 写笔记 & 跨设备同步

`colna add` 把一篇笔记写进 `memory/00-Inbox/`(自动加 `title` / `date` / `tags`
front-matter,文件名按标题生成且重名自动去冲突),写完顺手做一次增量索引。

`colna sync` 把跨设备同步收成一条命令:

1. 检查未完成的 Git 操作、冲突标记和非 `memory/` staged 文件;
2. `git add memory/` + 有改动则 commit;
3. `git pull --rebase --autostash` 拉远端最新并保留其它未提交改动;
4. 对最终真源状态 reindex 一次;
5. `git push`。

pull / rebase 失败会立即停止;pull 返回成功后还会复查 Git 中间态与 unmerged 文件,
避免 autostash 恢复冲突时继续 reindex / push。本地 Memo 提交仍保留,处理 Git 问题后可安全重试。

## 目录

```
memory/            真源 Markdown(入 git);PARA 结构,约定见 CONVENTIONS.md
  00-Inbox/        收件箱(colna add / clip 默认落这)
  10-Projects/     项目  20-Areas/ 领域  30-Resources/ 资源
  40-Archive/      归档  50-Daily/ 日记
  _templates/      模板  _attachments/ 附件(_ 开头不入索引)
.colna/            本地派生(不入 git)
  index.zvec       zvec 向量索引
  state.json       v5 增量状态(模型 / chunker 版本 + 文件 sha256 + chunk IDs/count)
  skills.zvec      技能索引(独立 collection)
  watches.json     watch-url 状态
  index.lock.d/    索引写入目录锁(含 PID)
src/
  chunker.rs       Markdown 切块 + front-matter + 标题面包屑 + 摘要块
  embedder.rs      fastembed 本地 E5 向量
  state.rs         增量索引状态(指纹 / 模型 / 切块版本)
  store.rs         zvec 封装(建库 / 写入 / 向量+FTS 检索)
  search.rs        过滤 / 时效 / 入链 / MMR / 多 query 融合
  indexer.rs       全量 / 增量索引 + 索引锁
  write.rs         put / append / update / archive(预览 + 乐观锁)
  context.rs       上下文包
  digest.rs        Daily 整理 / open work
  eval.rs          recall@k / MRR 评测
  graph.rs         笔记图谱 / backlinks
  ignore.rs        .colnaignore
  ingest.rs        PDF/HTML/图片/音视频摄取
  scope.rs         .colna/scopes.json
  secrets.rs       敏感信息扫描 / 脱敏
  skills.rs        技能索引
  task.rs          任务 receipt
  watch.rs         索引 watcher / URL 监控
  clip.rs          网页正文抽取
  rerank.rs        BGE cross-encoder 重排(可选)
  encrypt.rs       age 加密包装
  mcp.rs           MCP server(stdio JSON-RPC)
  gitsync.rs       colna sync(git 封装 + 自动 reindex)
  main.rs          CLI(index / search / context / clip / mcp / add / sync)
colna              运行包装脚本(自动设置 dylib 路径)
```

## 路线图

- [x] **P0**:md → 切块 → 本地 embedding → zvec → CLI 语义检索
- [x] **P1**:增量索引(按 hash 只重嵌入变化块)+ FTS 混合检索(RRF 融合)
- [x] **P2**:MCP server,Claude 直接调用 `kb_search` / `kb_get`
- [x] **P3**:`colna add` / `colna sync`(封装 git add/commit/push/pull + 自动 reindex)
- [x] **P4**:MCP 写接口 + 审核、检索过滤 / 时效 / MMR、索引新鲜度、`context`、`clip`
- [x] **P5**:digest / consolidate / open work、图谱 / backlinks、摘要块 + 长段切分、eval、附件摄取、查询扩展 / 重排
- [x] **P6**:watch / watch-url、task receipt、scope、`.colnaignore` / secrets / age、技能索引
