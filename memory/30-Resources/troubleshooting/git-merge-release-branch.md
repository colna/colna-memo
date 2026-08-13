---
title: 合入 release 分支的 git 坑
tags: [git, sitin-next, merge]
---


## 合 release 分支前必先对齐远端 tip(2026-08-12)
- **坑**:本地 release/test-pwaX 落后远端,`git pull --ff-only` 因分叉失败;若无视继续 `git merge <feature>`,会在过时基线上产生 merge + 解一堆冲突,最后 push 仍被拒(远端领先),白解。
- **正解**:先 `git fetch` → `git reset --hard origin/release/test-pwaX`(丢弃过时/错误的本地状态)→ 再 `git merge --no-ff origin/<feature>`。若远端已合过该 feature 的旧版,只会合新增提交,通常干净无冲突。
- **判断已合到哪个版本**:`git show <remote-merge> -s --format=%P` 看第二个父(被合的 feature 提交)。`git merge-base --is-ancestor <mycommit> origin/release/test-pwaX` 判断我的提交是否已在远端。
