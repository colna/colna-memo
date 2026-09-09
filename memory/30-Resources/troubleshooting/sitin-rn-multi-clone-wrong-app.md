---
title: sitin-rn / sitin-rn2 双 clone 跑错目录 → pnpm script not found
date: 2026-09-09
tags: [git, pnpm, sitin-rn, monorepo, 排错]
---

# pnpm 报 script not found 时,先怀疑「在哪份 clone」

**坑**(2026-09-09):在 `sitin-rn2/` 跑 `pnpm luka:ios`,报

```
ERR_PNPM_RECURSIVE_EXEC_FIRST_FAIL  Command "luka:ios" not found
Did you mean "pnpm lumi:ios"?
```

第一反应是「这个分支没配 luka」,实际配置完好,只是**跑错了 clone**。

**根因**:工作区有两份同 remote(`presence-io/sitin-rn`)的 clone,各自停在不同分支、`apps/` 内容不同:

| clone | 分支 | apps/ |
|---|---|---|
| `sitin-rn` | `feature/luka-ios` | blueprint / demo / intro / iris / koda / luka / lumi / naya / poo |
| `sitin-rn2` | `feature/koda-android` | demo / intro / iris / koda / lumi / naya |

根 `package.json` 的 `<app>:ios` 脚本是**逐 app 手写**的(`pnpm --filter <app> ios`),所以 app 目录没进这份 clone/分支,脚本自然也不在。pnpm 的 "Did you mean" 是拿现有 scripts 做的近似匹配,`luka` → `lumi` 只是**字面像**,毫无因果关系,极易误导。

**识别信号**:pnpm 提示的建议名是**另一个 app 的同名脚本**(luka→lumi、koda→demo 这类),而不是拼写笔误的修正 → 几乎一定是走错目录/分支。

**修法**:
```bash
git branch --show-current   # 我在哪个分支
ls apps/                    # 这个 app 在不在
```
确认后 `cd` 到正确的 clone 再跑;确实要在另一份 clone 干活,得 `git fetch && git checkout <branch> && pnpm install`。

**相关**:[sitin-next / sitin-next3 双 clone 同名分支过时坑](sitin-next-multi-clone-stale-branch.md) —— 同一类「多 clone」问题的另一种表现(同名分支 head 不一致)。
