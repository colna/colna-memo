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

## `business-pwa-proto` 的 dist 过期 → app-pwa 报一堆 proto 类型 `unknown`(2026-07-16 两次误判)

**现象**:`app-pwa` 跑 `tsc --noEmit` 突然几十个错,形如 `Cannot find module '@heyhru/business-pwa-proto/gen/archat_api/msgcenter_api'`,或 proto 类型全变 `unknown`(`Property 'code' does not exist on type 'unknown'`)。

**根因**:`app-pwa` import 的 `@heyhru/business-pwa-proto/gen/...` 指向 **dist**,不是 src。**dist 没构建/过期**就全线解析失败。触发场景:base 合入了用新 proto 的代码(如 `chatApi.ts` 用 msgcenter)、或 `git stash` / 分支切换让 dist 失效。

**修法(一条)**:

```bash
cd packages/business-pwa-proto && pnpm build     # 只重建 dist
```

**判据**:app-pwa 大量 proto 类型报错 = **先 rebuild dist,再怀疑自己的代码**。顺手拉基线对照(`git stash` 后跑 base)能立刻证明与改动无关。

### ⚠️ 别手贱跑 `pnpm proto:gen`

`src/gen` 是**入库的**(路径是 `src/gen`,不是 `gen`)。重新生成会因 protoc 版本差异 **churn 20+ 个受版本影响的入库文件**,把真实 diff 淹掉。回退:`git checkout -- packages/business-pwa-proto/src/gen`。

只有 proto submodule 指针真的变了才需要 `proto:gen`;日常缺 dist 只用 `pnpm build`。

> **踩坑教训**:我当时用 `git ls-files packages/business-pwa-proto/gen` 查返回空,就断定「gen 不入库」——**路径查错了**。`git ls-files <path>` 返回空只能说明**这个路径**没文件,不能推出「这类文件不入库」。先 `ls` 或 `git ls-files | grep` 确认路径。

> `protoc` 若报 not found,先看 `/opt/homebrew/bin/protoc`(装了但不在非交互 shell 的 PATH 里),别急着 `brew install`。

## 分支内部有「后一个 commit 推翻前一个」时 → 先 squash 再落,别直接 rebase

2026-07-16 PR #633:3 个 commit 里第 1 个引入 `margin-bottom` 压矮、第 2 个又删掉它。base 前进后直接 rebase,**要为那个死中间态解两遍冲突**,且该状态从未被验证过。

**改用只对最终状态解一次冲突**:

```bash
git checkout -b tmp origin/<base>
git merge --squash <branch>     # 冲突只解一次(最终状态)
# 解冲突 → commit
git branch -f <branch> tmp && git checkout <branch> && git branch -D tmp
git push --force-with-lease --no-verify
```

净 diff 不变,评审也不用看已被推翻的方案。**判据:分支内自我推翻 → rebase 的工作量和出错面翻倍,先 squash。**

## 验 gitignore 用 `check-ignore`,别用 `git status`(通用 git 技巧)

删文件 + 加 ignore 后想验证规则是否生效,`git status --porcelain --ignored <path>` **会被暂存的 `D` 记录盖住**,看不出结果。直接问规则:

```bash
$ git check-ignore -v .serena/project.yml .serena/cache/x
.gitignore:38:.serena/	.serena/project.yml
.gitignore:38:.serena/	.serena/cache/x
```

输出 `<规则文件>:<行号>:<规则> → <路径>`,一目了然;无输出 = 未命中。

## ⚠️ gitignore 里的文件却被跟踪 = **事故,不是约定**(2026-07-17 踩过)

`sitin-next` 的 `.gitignore:2` 有 `dist/`,但 `packages/app-ins-scripts/dist/snapchat/automation.js` 却是被跟踪的。我看到分支上最近两个 commit(`36293fb9d`、`8332eea31`)都带着它,判断「这是本分支的既有约定」,于是 `git add -f` 跟着提交 —— **错了**。

真相(一条命令就能拆穿):

```bash
$ git ls-files | grep -c "/dist/"          # 全仓被跟踪的 dist 文件数
1                                          # ← 只有它一个
$ git ls-files "packages/app-ins-scripts/dist/instagram" | wc -l
0                                          # ← 同一个包的 IG 产物根本没跟踪
```

它最早由 `8332eea31` 带进来 —— 那是个**改 sendStory DOM selector 的 `fix:` 提交**,dist 显然是 `add -f` / `add -A` 时误加的。而且**没有任何东西从 git 读它**:`upload-all.mjs` 是从 `src/` 上传脚本,`ig-script-tester` skill 读的 `dist/ig` 本来就不在版本库。纯本地构建产物。

**判据速记**:

> 看到「gitignore 忽略的路径却被跟踪」,**先假设是事故**。断定成约定之前必须查**同类对照**:同仓/同包的兄弟目录(IG vs SC)是不是也这样?全仓这类文件有几个?有谁真的读它?
>
> **从 2~3 个 commit 推断约定是不可靠的** —— 误提交也会被后续 commit 一路带着走,看起来就像约定。

## 取消跟踪后,别人(和你自己)切分支/pull 时工作区那份会被删

`git rm --cached` 后,切到新 HEAD 时 git 会把该文件从工作区删掉(它变回未跟踪 + 被 ignore)。2026-07-17 实测:merge 后 `git checkout personal/zz/sp-snapchat`,`dist/snapchat/automation.js` 直接消失。

构建产物无害,`node scripts/build.mjs` 重新生成即可;但**要提前告知团队**,否则别人 pull 完发现产物没了会懵。同源结论见 [[../../50-Daily/2026-07-16]] 里 `.serena` 那三层判断(合并的分支会删 / 其他分支不会 / git 历史永远都在)。
