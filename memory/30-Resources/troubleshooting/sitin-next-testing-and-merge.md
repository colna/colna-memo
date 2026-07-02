---
title: sitin-next 测试框架分歧 & 长期分支 merge 坑
date: 2026-07-02
tags: [troubleshooting, sitin-next, testing, jest, vitest, git, merge]
---

# sitin-next 测试框架分歧 & 长期分支 merge 坑

## app-social-proxy-server 用 Jest,不是 vitest
- 该包用 `jest.fn()` + `jest.config.js`,`test` = `prisma generate && jest`,**包内无 vitest 配置**。直接 `vitest run` 会 `describe is not defined`。
- 跑单个 spec:`pnpm --filter @heyhru/app-social-proxy-server exec -- jest <path>`(新版 jest 是 `--testPathPatterns` 复数,或直接传文件路径)。
- 对比:monorepo 其它包普遍 vitest。改哪个包先确认它用哪套。

## 功能分支往「落后的 release 分支」merge,极易带一整段无关历史
- 场景:`personal/zz/sp-script-error-csv-export`(=`feature/sp` + 1 个 CSV commit)要并进 `release/test-sp`,而 test-sp **落后 feature/sp 42 个 commit**、反向也有 8 个独有 commit → 真分叉。整分支 merge 会把 feature/sp 那 42 个无关 commit 全带进 test-sp。
- **发前必做**:`git log --oneline <target>..<mybranch>` **正反都看**,判断会带多少 commit / 是否分叉。
- **决策**:只要目标功能 → **cherry-pick 目标 commit**;确实要让 release 追平上游 → 才整分支 merge(需跟用户确认,如本次 test-sp 追平 feature/sp+CSV)。

## 把某服务改动从「混合分支」拆出来
- 最干净:**定位到隔离的单 commit → 从目标 base 切分支 → cherry-pick**(如 social-proxy 改动全在 `d81bd05b`,从 `feature/sp` 切分支 cherry-pick → PR)。
- 反向从源分支**移除**该 commit:`git rebase --onto <commit>~1 <commit> <branch>` 把其上的 commit 重放到 base、丢弃中间 commit,`--force-with-lease` 推。强推重写源分支不影响已 merge 进 release 的老节点(通过 merge 节点仍可达)。
- 若改动散在多 commit 才考虑 `git checkout <base> -- <paths>` 手工摘。

## 反复冒出来的 submodule 噪声
- `packages/business-pwa-proto/proto` 子模块指针经常显示 modified(非本人改动),挡 rebase/checkout。复位:`git checkout -- <submodule> && git submodule update <submodule>` 再操作。
