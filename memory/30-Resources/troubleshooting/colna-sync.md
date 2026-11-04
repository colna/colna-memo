---
title: colna sync 排错
date: 2026-06-25
tags: troubleshooting, colna-memo, git
---

# colna sync 排错

## 当前 Git / SSH 约定(2026-07-31)

- 工作区仓库使用**本机 SSH 配置与密钥**,遵循仓库现有 remote(通常为 `git@github.com:<owner>/<repo>.git`)。
- 不强制改写为 `github-colna` alias,也不再依赖工作区专用 `includeIf`;提交前按仓库实际生效配置核对 `git config user.name` / `git config user.email`。
- 下文 `includeIf` / `github-colna` 相关内容是旧机器上的历史排错记录,不代表当前执行约定。

## pull 被拒:cannot pull with rebase: You have unstaged changes

- **现象**:`colna sync` 输出 `(跳过 pull:... cannot pull with rebase: You have unstaged changes)`,远端未真正拉取对齐。
- **根因**:旧版 `run_sync` 顺序是「pull --rebase → reindex → add/commit → push」。`git pull --rebase` 要求工作区干净,而此时 memory/ 的本地改动还没 commit,rebase 被拒;代码用 `match ... Err => 友好提示` 把失败吞掉,所以只是"跳过 pull",不报错。
- **修法**(`src/gitsync.rs` run_sync):
  1. 把 **add + commit memory/ 提到 pull 之前**,保证 pull 时工作区干净。
  2. pull 加 **`--autostash`**(`git pull --rebase --autostash`),兜底任何残留的非 memory 未提交改动(如源码改动),rebase 后自动恢复。
  3. reindex 由两次合并为一次,放在 commit + pull 之后(真源状态稳定再建索引)。
- **教训**:`git pull --rebase` 对脏工作区零容忍;封装同步命令时「先提交本地、再拉远端」是更安全的顺序,`--autostash` 是廉价兜底。

## autostash 恢复冲突仍返回成功(2026-07-31)

- **现象**:`git pull --rebase --autostash` 已拉取成功,但恢复本地未提交源码时产生 `UU`;Git 仍可能返回 0。旧流程会继续 reindex / push 并打印同步成功。
- **修法**:pull 后再次运行 Git 中间态与 unmerged 检查;发现 autostash 恢复未完成就停止 reindex / push。回归测试同时覆盖二次 sync 守卫和 `rebase --abort` 后恢复本地 Memo / dirty source。
- **教训**:Git 命令退出码 0 只证明 pull 主操作成功,不证明 autostash 已无冲突恢复。

## 同一 Daily 跨环境内容分叉,rebase 撞真实冲突(2026-07-31)

- **现象**:两台环境(如本机 sitin 工作区 与 metabot-workspace/colna WORK)同日各自往 `50-Daily/YYYY-MM-DD.md` 追加**不同的工作日志条目**,`git pull --rebase` 把本地那条 commit 往远端上 replay 时,同一文件出现 `<<<<<<< HEAD` 内容冲突(不是 autostash 冲突,是提交内容本身分叉)。远端还可能顺带更新了几个项目/troubleshooting 笔记,本地是旧版。
- **安全合并手法(保留双方,不回退别人)**:
  1. `git rebase --abort` 回到干净态(会 `Applied autostash` 恢复本地 dirty 源码)。
  2. `git fetch origin main`。
  3. `git reset --soft origin/main` —— HEAD 移到远端;此时本地那条 commit 的**全部**改动变 staged,会暴露出你**没打算改**的文件(远端更新过、你本地是旧版的项目笔记),**别直接提交否则回退别人**。
  4. `git restore --staged .` 全部 unstage;`git checkout origin/main -- memory/` 把 memory/ 工作树整体对齐远端(丢弃本地对非目标文件的旧版),`src/*.rs` 等非 memory 的他人 dirty 源码不受影响。
  5. 只把你自己的 delta(本次要追加的 Daily 条目)用 Edit 重新加回文件末尾 → `git add 该 Daily` → commit → push。
- **教训**:跨环境写同一 Daily 必然分叉。**不要靠 rebase/autostash 自动合内容**;以远端为底 + 只重放自己的增量条目最稳。软重置后务必检查 staged 里有没有「你没改却被带上」的文件,那些是别人在远端的更新,要 checkout 远端版盖回,不能提交本地旧版。

## 全量索引内存膨胀与假完成(2026-07-31)

- **根因**:`fastembed 4.9.1` 默认 batch 256,对 batches 使用 Rayon 并发;每个 ONNX Session 又启用全部 CPU 线程。1355 chunks 会同时跑多批推理,曾达到约 29.5 GB footprint / 28.6 GB swap。旧代码还会一次向 zvec 写 1355 docs,超过其 1024 单批边界。
- **修法**:应用层按 16 chunks 串行执行 embedding + zvec 写入,并显式给 fastembed `Some(batch.len())`;真实全库 108 files / 1355 chunks 实测 107.35 秒、最大 RSS 2.04 GB、swap 0。
- **完整性**:state v2 记录格式版本、文件 hash、每文件 chunk IDs 和总数;写完 `flush → optimize → flush`,再验证 write counts、doc_count、schema 中 `text` / `embedding` 索引存在性和 vector completeness。zvec v0.5.0 的 `CollectionStats.indexes` 只列向量索引,不能用它判断 FTS 是否存在;FTS 需用 `Collection::schema().has_index("text")` 单独校验。state 缺失 / 损坏 / 旧版、索引打不开或数量 / 必需索引不符一律全量恢复,中断时不落 state。
- **删除坑**:zvec v0.5.0 `delete_by_filter` 在当前 HNSW + FTS schema 的优化后 collection 上会成功返回但 doc_count 不变;改为按 state 中的 chunk 主键删除,`NotFound` 视为幂等成功。

## 提交前冲突标记守卫(2026-06-26 新增)

- **背景**:一次 rebase 半途 `rebase --continue` 把带 `<<<<<<<`/`>>>>>>>` 冲突标记的 Daily 文件提交进了版本库,`colna sync` 不校验、照常 push 出去,污染真源。
- **修法**(`src/gitsync.rs`):新增 `check_conflict_markers()`,在 reindex 后、`git add`+commit 前调用。用 `git grep -n --untracked -e '^<<<<<<<' -e '^>>>>>>>' -- memory`(覆盖未跟踪文件);退出码 0=命中→`bail!` 中止(未提交未 push)、1=干净、其它=真错。不能复用 `git()` helper,因为 git grep 无匹配时退出码 1 会被误判为失败。
- **教训**:封装 sync 命令要在「写入版本库前」做一道脏数据守卫;冲突标记只查 `<<<<<<<`/`>>>>>>>`(distinctive),不查 `=======`(markdown setext 标题误伤)。

## colna-memo 的提交作者不是 colna(2026-07-09 发现,未修)

- **现象**:`colna sync` 产生的提交,作者是 `MacBook <max@MacBooks-MacBook-Air.local>`,不是 colna。最近 12 个提交全是。
- **根因**:`colna-memo` 仓库**没有配 `user.name` / `user.email`**,回落到系统默认。

  | 仓库 | 身份来源 | 生效作者 |
  |---|---|---|
  | `sitin-next` | 自己的 `.git/config` | `colna <richardzhang1999@163.com>` ✓ |
  | `colna-memo` | 无配置 → 系统默认 | `MacBook <max@…>` ✗ |

- **`CLAUDE.md` 描述的机制不存在**:它说本工作区靠 `~/.gitconfig` 的 `includeIf` → `zhangzheng/.gitconfig` 统一身份。
  实际 `~/.gitconfig` 里**只有** `includeIf "gitdir:~/Dev2/buchuan/"` 一条,**没有 zhangzheng**。
  `sitin-next` 之所以正确,是因为身份直接写死在它自己的 `.git/config` 里,与 includeIf 无关。
- **已修(2026-07-09)**:给 `~/.gitconfig` 补上 —— 真正实现 CLAUDE.md 描述的机制:

  ```gitconfig
  [includeIf "gitdir:~/Dev2/zhangzheng/"]
      path = ~/Dev2/zhangzheng/.gitconfig
  ```

  生效后 `colna-memo` / `sitin-next2` / `sitin-demo-webapp` 都从 `Dev2/zhangzheng/.gitconfig` 取身份;
  `sitin-next` 仍走自己的 `.git/config`(仓库级优先级更高,值相同)。`buchuan` 工作区不受影响。
  另修正 `zhangzheng/.gitconfig` 注释里写错的路径(`/Users/user` → `~`)—— 那正是这次误判的源头。
- **历史提交的作者不改**(需要 filter-branch / rebase 重写历史,得不偿失)。
- **教训**:**「约定写在 CLAUDE.md 里」不等于「机制真的生效」。** 涉及身份/凭据的约定,要用 `git config --show-origin` 之类**查生效值**,别信文档。

## 坑:sync 报「dimension mismatch, expected 768 but got 384」(2026-08-03)

- **现象**:`./colna sync` 重新嵌入时报 `zvec error InvalidArgument: field[embedding] dimension mismatch, expected 768 but got 384`,索引无法写入。
- **根因**:`./colna` 包装脚本默认 `COLNA_PROFILE=debug`,而 `target/debug/colna` 是**升级 e5-base 之前**的旧编译(e5-small,384 维);zvec 索引已由 release 二进制重建成 768 维(e5-base)。旧 debug 二进制产出 384 维向量 → 与 768 维索引字段冲突。
- **修法**:`COLNA_PROFILE=release ./colna sync`(release 二进制是 768 维,与索引一致)。
- **一次性彻底修**:`cargo build`(重编 debug 到 e5-base 768),或把包装脚本默认 profile 改成 release。
- **教训**:embedder.rs 改了模型/维度后,**debug 与 release 两个二进制都要重编**,否则用哪个过期就哪个爆维度不一致。

## `colna sync` 必须在知识库根目录跑,否则写错仓库 + 扔下 1.1 GB 缓存(2026-09-11)

- **现象**:在**别的仓库**(如在 `sitin-rn/`)直接调 `"/Users/colna/WORK/colna-memo/colna" sync`,报
  `Error: git ["add", "--", "memory"] 失败: fatal: pathspec 'memory' did not match any files`。
- **根因**:包装脚本按**当前工作目录**拼 git 命令(`memory/` 相对路径)。cwd 不是 KB 根就找不到 `memory/`,
  commit/push 整条流程中止。更隐蔽的是:embedder 也会把 fastembed 的模型缓存落到 cwd —— 一次误跑就在
  `sitin-rn/` 根留下 **1.1 GB 的 `.fastembed_cache/`**(KB 自己那份 1.5 GB 在 `colna-memo/.fastembed_cache`),
  而它**没被 gitignore**,`git add -A` 会连锅端。
- **修法**:一律 `cd /Users/colna/WORK/colna-memo && ./colna sync`。`COLNA_PROFILE=release` 那条约定的前提也是
  「站在 KB 根」。
- **自查**:sync 报错后先 `pwd`;另可 `ls "$PWD/.fastembed_cache"`,非 KB 目录里出现它,就是误跑的痕迹(删之前先确认
  KB 下已有一份)。

## E5 模型升级后的 state 版本迁移(2026-08-05)


- **风险**:只升级 `embedder` 的模型/维度而不改变 `.colna/state.json` 格式时,旧 384 维索引的 state 仍可能通过文件 hash、chunk 数量校验,被误判为可增量使用。
- **修法**:把 `IndexState::FORMAT_VERSION` 从 v2 升到 v3,旧 state 自动触发干净的全量重建;重建后再执行 `flush → optimize → stats` 校验。
- **教训**:索引 state 不只是文件增量缓存,还要视为 embedding 模型与 chunking 规则的兼容性指纹;模型或嵌入输入契约变更必须 bump 版本并配回归测试。

## 半途 kill 会留下孤儿 index 进程（2026-09-17 实测）

- **现象**：`colna sync` 在前台被超时/中断（例如 agent 工具 180s 超时、Ctrl-C）后，同步进程本身被杀，但它拉起的 `colna index` 子进程**没有一起走**（实测 `ps` 里同时有 300%+ CPU 的 `colna index` 与 `colna sync` 两个进程）。
- **后果**：两个进程抢同一个 `.colna/index.zvec` 库 —— 重建日志里出现 `Index file ... already exists (possible crash residue); cleaning and overwriting`，进度退化到约 **1 batch / 分钟**（16 块一批，183 批要 ~3 小时；单进程全量重建本来只要几分钟）。
- **修法**：`pgrep -fl "colna (sync|index)"` 查有没有孤儿；有就 `kill` 掉，只留一个进程重跑。下次前台超时就别等，先查进程再重跑。
- **判据一句话**：sync 的耗时以「单进程全量重建几分钟」为基准，明显超出就是有并发进程在打架。

## 并发 sync 持锁时新进程会静默等待(2026-09-21 实测)

- **现象**:`./colna index`(或 `sync`)长时间**零输出**,像卡死;`pgrep -fl colna` 只有一个正常进程,且它**在健康推进**(`/tmp/colna-sync.log` 里 `batch N/202` 约 5 秒一批,202 批 ~17 分钟)。
- **根因**:zvec 库有排他锁(`.colna/index.zvec/LOCK`、`idmap.0/LOCK`、`fts.1.rocksdb/LOCK`)。本工作区常有并行会话同时在 sync,后启动的进程在锁上**静默阻塞**:不报错、不输出进度,看起来与死锁一样。
- **判据 / 修法**:先 `pgrep -fl "colna"` + `lsof -p <pid> | grep LOCK` 分清两种情况 ——
  - 持锁者**健康重建**(有输出、batch 在涨、CPU 在跑):**等它**,不要 kill 也不要重跑(会重演上一条的抢占退化)。
  - 持锁者**僵尸/孤儿**(CPU 空转、batch 不动):按上一条 kill 后单进程重跑。
- **另一条实用结论**:`colna sync` 的顺序是 `add+commit → pull → reindex → push`。reindex 是全流程最长的一步,agent 工具超时 kill 后,**commit 已经产生但 push 还没发** —— 表现为「本地 ahead 若干个 `colna sync` 提交」。补推只需在 KB 根 `git push origin main`(已实测 7 个积压提交一次推完)。
- **教训**:判断 `colna` 死活看**输出速率**而不是「有没有输出」;看到 ahead 一堆 `colna sync` 提交,八成是历次 sync 都死在 reindex 上。

## `git pull --rebase` 报 `Cannot rebase onto multiple branches`（2026-11-04 实测）

- **现象**：`colna sync` 的 pull 步报 `fatal: Cannot rebase onto multiple branches.`（当日一次中断后遗留 `FETCH_HEAD` 多行），commit 已完成、reindex / push 未做。
- **根因**：`remote.origin.fetch = +refs/heads/*:refs/remotes/origin/*` 是**通配 refspec**，裸 `git pull --rebase` 不指定上游时会按**整个 `FETCH_HEAD`** 推断上游；一旦 `FETCH_HEAD` 残留多行，就变成「多个上游分支」而中止。**不是配置损坏**：`.git/config` mtime `Aug 5 16:04`（当日无人改动），`git fetch` 后 `FETCH_HEAD` 只有一条 `branch 'main'`，错误即消失（一次性残留）。
- **修法（历史注记，已被 17:12 的定稿取代，勿照抄）**（`src/gitsync.rs:147-158`，2026-11-04 16:44 用户批准）：当时把 pull 显式写上游 `git pull --rebase --autostash origin main`；后续实测证明这只是**过渡修法，不免疫**多行 `FETCH_HEAD`（见下文「更正」与「定稿」），现已换成 `fetch + rebase --autostash origin/main`。
- **回退**：把 `src/gitsync.rs:154` 的 `["pull","--rebase","--autostash","origin","main"]` 去掉末尾两个元素、`src/gitsync.rs:153` 的提示串还原为 `git pull --rebase --autostash ...`，再 `cargo build`；全程不改 `git config`（`remote.origin.fetch` 保持通配原样）。
- **代价**：分支名 `main` 被硬编码；本库与测试仓库都约定 `main`，换分支的仓库需同步改这一行。
- **教训**：`FETCH_HEAD` 是每次 fetch 覆盖的临时文件，但残留会污染「不指定上游」的 pull；跨设备同步脚本应显式声明上游，别依赖推断。

## `Can't lock read-write collection`：两条 colna 命令重叠（2026-11-04 16:44 实测）

- **现象**：`sync` 还在跑时又起了一条 `index`，后一条报 `Can't lock read-write collection`；下一条命令自愈。
- **根因**：同一 `.colna/index.zvec` 库的排他锁被前一条命令持有 → 两条重叠即撞锁。**不是并发残留锁**：`.colna/index.zvec/LOCK` 是 `Oct 30 18:46` 的**陈旧空文件**，锁目录 `index.lock.d` 当时不存在，持锁进程也已不存在。
- **判据 / 修法**：先 `pgrep -fl "colna (sync|index)"` 确认有两条命令在跑（而不是一条健康进程在持锁），然后**串行**重跑；不要为这条报错去删 `LOCK` 或 `index.lock.d`。
- **教训**：别把 `sync` 和 `index` 并排丢进同一个 shell（`cmd1 & cmd2` 或两行同时发）；`LOCK` 存在本身不代表有进程持锁，要看进程。

## 更正（2026-11-04 16:56 实测）：显式 `origin main` **并不能**根治上面那条报错

- **反例**：同一台机器上 `git pull --rebase --autostash origin main` 报 `fatal: Cannot rebase onto multiple branches.`，`PULL_EXIT=128`，当时 `wc -l .git/FETCH_HEAD` = **2**。触发条件是我**两条 git 命令重叠**（前一条 `git fetch --update-head-ok origin main` 还在跑），两次 fetch 各写一行 → `FETCH_HEAD` 两行。
- **受控复现**（git **2.54.0** Apple Git-157，`/tmp` 裸仓库）：`git pull --rebase --autostash origin main main`（两个 head，**指向同一个 commit**）→ `fatal: Cannot rebase onto multiple branches.`，退出码 **128**；同样的命令换成单 head → `Already up to date.`。
- **结论修正**：`pull --rebase` 判定上游靠的是 **`FETCH_HEAD` 里「可合并」（未被 `not-for-merge` 标记）的行数**；显式写 `origin main` 只消除了「按默认 refspec 推断上游」这条路径，**行数 >1 时照 die**，与两条是不是同一个分支/同一 commit 无关。故上一条「显式 `origin main` 可根治」的表述**撤回**，正确说法是「消除推断路径，消除不了多行 `FETCH_HEAD`」。
- **根治写法（实测不读 `FETCH_HEAD`）**：`git fetch origin main && git rebase --autostash origin/main` —— 在同一份「人为造出的 2 行 `FETCH_HEAD`」状态下退出码 **0**，因为它 rebase 到 remote-tracking ref。代价是把一步 pull 拆成两步，且 rebase 失败时 autostash 恢复语义要自己守住（现有 `ensure_no_git_operation` 守卫仍适用）。
- **触发条件就是并发**：两次 fetch / pull 重叠即可写出两行。两条防线：① **串行化**（不要并行跑 `sync` / `index`，与下一条撞锁同源）；② 要彻底免疫就把 sync 的 pull 换成 `fetch + rebase origin/main`。
- **判据一句话**：看到这条报错先 `wc -l .git/FETCH_HEAD`——是 1 行，查其它原因；是 2 行以上，就是并发 fetch 撞的，改写法而不是改 config。
- **量化补充（2026-11-04 17:05，`/tmp` 裸仓库）**：同一脚本并发跑两次 `git fetch origin main`，连续 **30 轮 → 30/30** 全部得到 **2 行 `FETCH_HEAD`**。即「两行」不是偶发残留，而是**并发 fetch 的稳定结果**；只有串行执行时才是 1 行。故归因写成「**并发 fetch → 多行 FETCH_HEAD → `--rebase` 型 pull 必 die**」，「一次性残留」只是它在**事后**的表现（进程早已退出，行还留着）。

## 定稿（2026-11-04 17:12，用户 17:05 授权）：免疫写法 = `fetch origin main` + `rebase --autostash origin/main`

- **定稿结论**：**并发 fetch → 多行 `FETCH_HEAD` → `--rebase` 型 pull 必 die**。免疫写法是把拉取拆两步：`git fetch origin main`（只抓单分支）→ `git rebase --autostash origin/main`（重放到 remote-tracking ref，**完全不读 `FETCH_HEAD`**）。
- **证据 1（并发量化，17:05）**：同一 `/tmp` 裸仓库并发跑两次 `git fetch origin main`，连续 **30 轮 → 30/30** 得到 2 行 `FETCH_HEAD`；串行只有 1 行。
- **证据 2（受控复现，17:12）**：`git pull --rebase --autostash origin main main`（2 个 head、同一 commit = `FETCH_HEAD` 2 行可合并）→ `fatal: Cannot rebase onto multiple branches.`，**退出码 128**；同一状态下改后代码路径（`colna sync`：fetch 后台再造一行 → 2 行 `FETCH_HEAD`）→ **`SYNC_EXIT=0`**，日志 `⬇️ git fetch origin main ...` / `⬇️ git rebase --autostash origin/main ...` / `✅ 已 rebase 到 origin/main:7e05937 → 74efd8f` / `✅ 同步完成。`
- **实现**（`src/gitsync.rs:147-197`，提交 `4708c03`，`src/gitsync.rs` **+71/−21**、`src/main.rs` **+1/−1**）：fetch 失败 → 打印 git 原始 stderr（不截首行）+ 非零退出，不进入 rebase；rebase 冲突 → 打印冲突文件清单 + 非零退出，**不自动 abort / 不自动 continue**（现场保留）；rebase 无新提交 → 输出 `✅ 已是最新:origin/main …(HEAD <sha>)` 再继续 reindex；`rebase_in_progress()`（`:225`）区分「rebase 冲突」与「autostash 恢复冲突」两种报错。回归 `cargo test gitsync` → **5/5 PASS**。
- **回退**：`git revert 4708c03`；或把这两步还原成单条 `pull --rebase --autostash` 再 `cargo build`。全程**不动** `git config`（`remote.origin.fetch` 保持通配 `+refs/heads/*:refs/remotes/origin/*`）。
- **代价 / 约束**：分支名 `main` 硬编码在两处调用里；换分支的仓库要同步改。`pull` 自带的「重抓一次」保护也没了 —— 但那份保护正是失败模式的一半（它的重抓与别的进程重叠时才写出两行），拆开后每次只抓单分支，反而确定。
- **判据一句话**：看到 `Cannot rebase onto multiple branches` 先 `wc -l .git/FETCH_HEAD`；≥2 行就是并发撞的，**该换写法，不是改 config、也不是加显式分支**。

## 坑：`Permission denied (publickey)` 可能是**网络层 TLS 被重置**的假象（2026-11-04 17:37–17:46 实测）

**症状**：`./colna sync` 提交成功、卡在 fetch：`git fetch origin main 失败: git@ssh.github.com: Permission denied (publickey). fatal: Could not read from remote repository.` —— 看起来像 SSH key 丢了。

**为什么容易误判**：`~/.ssh/config` 把 `github.com` 映射到 `ssh.github.com:443`（绕 22 端口的常见配法），所以报错主机名是 `ssh.github.com`。此时三条「正常检查」全会误导你：

| 检查 | 结果 | 说明 |
|---|---|---|
| `ssh-add -l` | `The agent has no identities.` | **不是根因**——`id_rsa` 无 passphrase，本来就不依赖 agent |
| `nc -z ssh.github.com 443` | **通** | TCP 能建连，所以「端口不通」这条路查不出来 |
| `curl -sS https://ssh.github.com/` | **`SSL_ERROR_SYSCALL`** | **这才是真信号**：TLS 握手被中间网络重置 |

**根因**：`ssh.github.com:443` 的 TLS 被重置，ssh 拿到半死连接 → 最终报成 `Permission denied (publickey)`。而且它是**间歇的**：同一分钟内 `git ls-remote` 偶发成功、`git fetch` 连续失败，制造「刚才还行现在不行」的错觉。

**怎么判（两步定死）**：
1. `curl -sS -o /dev/null -w "%{http_code}" --max-time 8 https://ssh.github.com/` → 出 `SSL_ERROR_SYSCALL` / `code=000` = **网络层**，别再去折腾密钥或 `ssh-add`。
2. 复核 22 端口：`ssh -p 22 -o HostName=github.com -T git@github.com`。秒回 `Permission denied` = 22 可用但 key 真不行；若也超时 = 整个 GitHub SSH 面被墙，直接走 HTTPS。

**怎么绕（本次实测有效，且不改 `origin`）**：

```bash
git -c http.version=HTTP/1.1 push https://github.com/colna/colna-memo.git main
```

- `credential.helper=osxkeychain` 已有凭据，**不需要新 token**。
- **必须带 `-c http.version=HTTP/1.1`**：默认 HTTP/2 在这条链路上报 `Error in the HTTP2 framing layer`。
- 可能收到 GitHub `remote: Internal Server Error`（服务端 500，**不是本地问题**），重试即可。
- 重试时显示 **`Everything up-to-date`** 要读懂：说明**上一次其实已经推成功了**，不是「什么都没推」。
- 推到显式 URL **不更新本地 `origin/main`**，`git status` 会继续显示 `ahead 1`。补一次 HTTPS fetch 修 tracking：`git fetch https://github.com/colna/colna-memo.git main:refs/remotes/origin/main`。
- 权威确认远程：`git -c http.version=HTTP/1.1 ls-remote https://github.com/colna/colna-memo.git refs/heads/main`。

**善后（容易漏）**：`colna sync` 在 fetch 一步退出，**后面的 reindex 不会执行** → 必须手动 `./colna index`，否则新写的笔记搜不到。本次实测：变更/新增 3、删除 0、upsert **126** 块，索引 **351 文件**。

**判据一句话**：sync 报 `Permission denied (publickey)` 而 `ssh-add` 空、`nc` 通 → 先 `curl https://ssh.github.com/`；出 `SSL_ERROR_SYSCALL` 就是网络层，换 HTTPS 推 + 手动 reindex，别重配 key。
