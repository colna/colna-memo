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

## revert-and-reland:stacked PR 的 base 改到「已 squash 目标」要重建树,不能只改 base(2026-08-27)

**场景**:revert(#1027)先合进 feature/pwa,reland 分支(revert + revert-the-revert 历史)要提 PR 合回 feature/pwa。

- **坑**:reland 从旧 feature/pwa(还带 SP-V2)分出,当前 feature/pwa 用**独立 squash-revert** 去掉 SP-V2。直接 `gh pr edit --base feature/pwa` → **CONFLICTING**,且三点 diff 把 revert/revert-the-revert 抵消、只剩少数文件,合并时可能把 SP-V2 抵消丢掉。
- **修法**:reland 的**树本来就是对的**(两点 `git diff feature/pwa reland` = 目标内容)。用 `git commit-tree <reland树> -p feature/pwa -F msg` **重建成单提交挂当前 feature/pwa**,`--force-with-lease` 推个人分支。合并基变 feature/pwa,PR diff 干净、可 fast-forward。
- **通用规律**:stacked/revert 历史的分支要改基到「已独立处理过同内容的目标」时,**别只改 base(三点 diff 会抵消 + 冲突)**,直接 commit-tree 把正确的树重挂到目标上。

## reland 长期开着,base 会前进 → 定期 `git merge base`(2026-08-27)

reland PR 开着期间 feature/pwa 又合了别的 PR(如 #972 cashout_source)→ PR 变 CONFLICTING。修:worktree 里 `git merge origin/feature/pwa` 解冲突(showInsModal 签名冲突取 SP-V2 版 `{onAuthorized}` + 保留对方新参数),proto 随合并自动更新。

## worktree symlink 主 clone node_modules 跑 tsc:新 proto 类型是「假报」(2026-08-27)

worktree `ln -s 主clone/node_modules` 后,`@heyhru/business-pwa-proto` 解析到**主 clone 的别分支 proto**。若目标分支/合并后 proto 有新类型(如 #972 的 `CashOutSource`),tsc 会冒一堆「未导出」**假报**。**判真伪用增量对比**:`git stash` 改动 → 跑 pristine tsc → `comm -13`,只看增量;或对比合并后 worktree 自己的 `packages/business-pwa-proto` 是否含该类型(含=假报)。tsc 6.0.2 `-b` 撞 baseUrl 弃用,用 `tsc -p tsconfig.json --ignoreDeprecations 6.0`(app-pwa tsconfig 本就 noEmit)。

## 强拉齐非保护分支 + CI 会回推(2026-08-27)

`release/test-pwa2` 未受保护(`gh api .../branches/... --jq .protected`=false),colna push=true 即可 `--force-with-lease` 把它指到源分支 commit → 完全一致(同 SHA)。**坑**:force-push 后 `merge-to-release-test.yml` 之类 CI 可能自动再推提交、打破「完全一致」。核对用 `git ls-remote origin refs/heads/<branch>`(权威、单行),别信复合命令里的瞬时输出。

## gh 身份漂移:每次 gh 写操作前必查(2026-08-27)

本机 gh keyring 同时有 colna 和 buchuan1023,active 账号会被别处(lark 扫码等)切走。**只在会话开头查一次不够**——某次 `gh pr create` 用了 buchuan1023 身份建 PR(#1039,commit 身份仍 colna 正确)。GitHub 不能改已建 PR 作者 → 只能 `gh auth switch --user colna` + 关 PR + 重开(#1040)。**规矩:每次 gh pr create / 任何 gh 写前都 `gh api user -q .login` 确认 colna**。见 [lark-cli-auth-and-perms](lark-cli-auth-and-perms.md) / [global-flag-ownership-drift](global-flag-ownership-drift.md)。
