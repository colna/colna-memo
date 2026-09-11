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

**相关**:[sitin-rn 双 clone 跑错目录](sitin-rn-multi-clone-wrong-app.md)(另一类本地门禁的误报/误判)。
