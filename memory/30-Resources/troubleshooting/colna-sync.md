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

## E5 模型升级后的 state 版本迁移(2026-08-05)

- **风险**:只升级 `embedder` 的模型/维度而不改变 `.colna/state.json` 格式时,旧 384 维索引的 state 仍可能通过文件 hash、chunk 数量校验,被误判为可增量使用。
- **修法**:把 `IndexState::FORMAT_VERSION` 从 v2 升到 v3,旧 state 自动触发干净的全量重建;重建后再执行 `flush → optimize → stats` 校验。
- **教训**:索引 state 不只是文件增量缓存,还要视为 embedding 模型与 chunking 规则的兼容性指纹;模型或嵌入输入契约变更必须 bump 版本并配回归测试。
