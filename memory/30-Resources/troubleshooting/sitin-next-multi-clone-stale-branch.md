---
title: sitin-next / sitin-next3 双 clone 同名分支过时坑
date: 2026-08-29
tags: [git, sitin-next, worktree, 排错]
---

# sitin-next / sitin-next3 双 clone 同名分支 head 不一致

**坑**:工作区里 `sitin-next/` 和 `sitin-next3/` 是同一 GitHub 仓库(presence-io/sitin-next)的两个 clone。同名分支(如 `personal/zz/admin-pwa-benefit-pool`)的**本地 head 可能不同**——某个 clone 的本地分支落后于 origin/另一个 clone。

**具体**(2026-08-29):`sitin-next` 的该分支 head `be7db71` 是 `sitin-next3`=origin=PR #966 的 `3a99ba00` 的**祖先**,少 30 个提交。用 sitin-next 旧树回忆/分析代码 → 判定严重失真(把已实现的功能当成没做)。

**根因**:各 clone 独立 fetch,不会自动互相同步;哪个 clone 平时在这分支干活(本例 sitin-next3),哪个就最新。

**修法/预防**:
1. 分析或改动某分支前,先 `git -C <clone> fetch origin <branch>` 再 `git rev-parse origin/<branch>` 对齐,**以 origin head 为准**。
2. 认「哪个 clone 是该分支的实际工作副本」——PWA 运营池/分级 tier 相关在 **sitin-next3**(见 `10-Projects/pwa-benefit-pool.md`)。
3. 建 worktree 分析时,从**最新的那个 clone**或直接 `git worktree add <path> origin/<branch>`(用 origin ref)。
4. 改动需跑 lint/tsc → 直接在有 node_modules 的主 clone(sitin-next3)上切分支干活(scratchpad worktree 没装依赖,工具链跑不了);其未跟踪 docs 目录不影响切分支。
