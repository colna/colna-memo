---
title: 合入 release 分支的 git 坑
tags: [git, sitin-next, merge]
---


## 合 release 分支前必先对齐远端 tip(2026-08-12)
- **坑**:本地 release/test-pwaX 落后远端,`git pull --ff-only` 因分叉失败;若无视继续 `git merge <feature>`,会在过时基线上产生 merge + 解一堆冲突,最后 push 仍被拒(远端领先),白解。
- **正解**:先 `git fetch` → `git reset --hard origin/release/test-pwaX`(丢弃过时/错误的本地状态)→ 再 `git merge --no-ff origin/<feature>`。若远端已合过该 feature 的旧版,只会合新增提交,通常干净无冲突。
- **判断已合到哪个版本**:`git show <remote-merge> -s --format=%P` 看第二个父(被合的 feature 提交)。`git merge-base --is-ancestor <mycommit> origin/release/test-pwaX` 判断我的提交是否已在远端。

## "让分支树 == 目标分支,但保留 merge 历史"(2026-08-24, PR #994)
- **场景**:`feature/pwa-nationwide` 合入 `release/online-pwa-pre`,但需求是「pre 内容与 nationwide **一模一样**」——两条并行 pwa 线严重分叉(feature 领先 1630 / release 领先 276,cherry 191 个 release 提交 patch 不等价),`git merge` 直接爆 **101 冲突**(含聊天页 `pages/ChatDetail/`→`pages/Chat/` 整体重写产生的 14 个 UD)。
- **正解**(免逐个解冲突):
  1. `git merge --no-commit --no-ff origin/feature/pwa-nationwide`(拿到 MERGE_HEAD)
  2. `git read-tree -u --reset origin/feature/pwa-nationwide`(index+工作树整体覆盖成目标树,MERGE_HEAD 不受影响)
  3. `git submodule update --checkout <sub>`(对齐 submodule 工作树到目标记录指针)
  4. `git commit`(得**双亲 merge 提交**,但树 == 目标分支)
  5. 校验:`git diff <mergecommit> origin/feature/pwa-nationwide --stat` 必须 **0 行**
- **为什么不用别的**:`-s ours` 保留的是 our 树(反了);`git checkout <branch> -- .` 删不掉 our-only 文件、树不会完全一致。`read-tree --reset` 才能做到逐字节等于目标。
- **proto 生成物冲突**:别手 merge `src/gen/*`。取最新 proto 指针(feature 的 3884fef0 已含 release/online 且领先 310)→ `bash scripts/generate.sh` 重生成(见 [sitin-pwa-proto-surgical-gen](sitin-pwa-proto-surgical-gen.md))。用 read-tree 整体对齐时,gen 也会直接等于目标提交的 gen,重生成动作被覆盖但结论一致。

## release/online-pwa-pre 受保护,colna 无直推权(2026-08-24)
- **坑**:直推 `origin/release/online-pwa-pre` 被拒 `GH006 Protected branch update failed`。
- **正解**:merge 提交推到个人分支(如 `personal/zz/merge-*`)→ `gh pr create --base release/online-pwa-pre --head <个人分支>`。若 merge 提交的 parent1 就是 pre 当前顶端,PR 为**干净快进无冲突**。
- 附带收获:pre-push 钩子会跑完整 turbo build(7 task),被拒前已验证过构建,可当免费冒烟。
