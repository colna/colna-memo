---
title: sitin-rn / 注释里点名别的 app 会被 boundaries 拦下
date: 2026-09-11
tags: [sitin-rn, ci, git, monorepo, 排错]
---

# 注释里点名别的 app,会被 boundaries 判成串味

**坑**(2026-09-11):在 `apps/luka` 里给一条注释写「上一版是 `#EFF3FD` 的冷蓝直角条,那是 **blueprint** 的蓝」。
`git commit` 一路顺利(pre-commit 只跑 biome),`git push` 时被 pre-push 拦下:

```
✗ 2 identity leak(s):

  apps/luka/src/components/me/feed-composer-bar.tsx contains "blueprint" — that identity belongs to app "blueprint".
  apps/luka/src/components/me/feed-composer-bar.tsx mentions "blueprint" — that is app "blueprint"'s name.
```

**根因**:`scripts/ci/boundaries.mjs` 的串味扫描**不做语法分析,只做文本匹配** —— 它遍历 `apps/*` 下
`.ts .tsx .js .jsx .mjs .cjs .json .podspec .gradle .kt .java .swift .h .m .mm .rb .plist .xml`,
拿 `apps.json` 里每个 app 的 `id` 与 `name` 分别做「包含」和「词边界」两种匹配(所以同一个文件会报两条)。
**注释、JSDoc 里的行内说明一样算**,而「解释上一版长什么样」恰好是最自然的写法,正好踩在上面。

**修法**:换措辞,别点名别的 app —— 「那是随模板带过来的颜色」既保住了信息,又没有那个字符串。
**不要 `--no-verify`**:这条检查是防串包(多 App 共用一套域名/账号 ID/名字会被商店连坐,见
`sitin-rn/docs/app-review-strategy.md` §4.2)的第一道闸,绕过去等于把它关掉。

**自查**:`pnpm boundaries` 本地随时能跑(约 1s、不联网)。它和 `pnpm typecheck` 都在 **pre-push** 而不在
pre-commit —— 也就是说,写了一段「别的 app 怎么怎么样」的注释,只有到最后一次 push 才会炸,先手动开一枪。

**再犯（2026-09-16）**:全 app 审查修复批里给 `chat-billing.ts` 写「iris/lumi guard the same way」
（想表达兄弟包也这么判），再次被拦（`mentions "iris"`）。这次连 `apps/iris` 路径都不安全——同文件里
既有的 `apps/iris 的注释里记着…` 没被报是运气（匹配看词边界，`apps/iris` 带路径前缀），**别赌**。
替代写法：`the sibling apps' billing guards the same way` 或「兄弟包的扣费判定相同」——不出现任何
app 名字面量。注意 AIGC 生成的「参考 iris 的写法」类注释是高发区，写完先 `pnpm boundaries`。

**再犯（2026-09-23）**:Luka 埋点审计时给 `services/native/analytics.ts` 的 `signin_click` 写注释解释
「与共享目录拼写不一致，不改共享后缀，**Iris** 等其它 App 的 wire 不受影响」——又被 `mentions "iris"`
拦下。改法同前：写成「其它 App 的 wire 不受影响」。凡是解释「为什么不能进共享包 / 为什么要本地保留」时，
最容易顺手点名兄弟 app 当例子，这是第三类高发场景。

**相关**:[sitin-rn 双 clone 跑错目录](sitin-rn-multi-clone-wrong-app.md)(另一类本地门禁的误报/误判)。
