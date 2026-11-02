---
title: sitin-next SP 冷启动 abnormalByPlatform 恒 null —— 没进探测分支，不是探测失败
date: 2026-11-02
tags: troubleshooting, sitin-next, pwa, social-proxy, ins-bridge, abnormalByPlatform
---

# SP 冷启动 `abnormalByPlatform` 恒为 null

## 现象

`release/test-pwa` 冷启动时 `checkSocialProxyAbnormal()` **有执行**，但没真正走到 `checkSocialProxyPageAbnormal`，导致 `abnormalByPlatform` 一直是 `null`（IG/SC 两个平台都拿不到异常判定）。

## 定位口径：先分清「没进分支」还是「探测失败」

`checkSocialProxyPageAbnormal` **失败返回 `code:-1`，不是 `null`**；判据是 `r.code === 0 && r.type === 0`，失败即为 `false` → 会写 `abnormal = true`（`useInsTaskInit.ts:462/465` → `:471-478`）。

所以「恒 null」只可能是**根本没进 `:461/464` 的探测分支**，即卡在下面两道门之一：

| 门 | 位置（`packages/app-pwa/src/hooks/useInsTaskInit.ts`） | 行为 |
|---|---|---|
| 门 A | `:445-449` | `if (!isApp())` → `setSocialProxyChecking(false)` 后 **`return true`**，探测整段跳过（回填授权态已在其前完成） |
| 门 B | `:456-467` | `igAuthed/scAuthed = authByPlatform[platform] === true`；未授权则 `Promise.resolve(null)`，写回处 `!== null` 才 `setPlatformAbnormal` |

## 另一半成因：persist merge 把 null 灌回来

`insStore.ts:184-187` 的 persist `merge`：

```ts
merged.abnormalByPlatform = {
  instagram: merged.insPageAbnormal,   // 旧字段为 null 时 → instagram: null
  ...(p.abnormalByPlatform ?? {}),
};
```

即使运行期写过 `abnormalByPlatform`，落盘/回灌后「读出来就是 null」也可能来自这里，不能只怀疑探测链。

## 线上对号（唯一能区分的证据）

按日志两句先后：

- `[INS_BRIDGE] setPlatformAuth:`（`insStore.ts:97`）
- `[INS_BRIDGE] setPlatformAbnormal:`（`insStore.ts:125`）

| 日志表现 | 结论 |
|---|---|
| 有 `setPlatformAuth`、无 `setPlatformAbnormal` | 卡**门 B**（未授权 / 授权态未回填 true） |
| 两条都没有 | **门 A**（非 APK，或 `isApp()` 判定失败） |

## `abnormalByPlatform` 全量读写点（2026-11-02，`release/test-pwa`）

| 类型 | 位置 |
|---|---|
| 写 | `insStore.ts:124-129` `setPlatformAbnormal`、`:143-147` legacy `setInsPageAbnormal`、`:164` `resetInsState`、`:184-187` persist merge |
| 调用方 | `useInsTaskInit.ts:473/477`（冷启动探测）、`:966`（小崽拉起成功清异常）、`useInsBridgeCallbacks.ts:184`（授权成功清）、`:211`（断连标异常） |
| 读 | `insStore.ts:228`（`selectPlatformUsable`）、`showSocialAuthDrawer.tsx:287`（feature/pwa `:288`） |

## 分支归属：`feature/pwa` 有同样问题

- `useInsTaskInit.ts` 两侧 blob **同为 `d60e094a43f87585fa2e506532f4ef54d221f103`**，`git diff` 为空 → 门 A / 门 B / 写回三段**逐字相同**。
- `merge-base` = `845a2ed7f`（即 `feature/pwa` HEAD），`rev-list --left-right --count` = `0/18`：`feature/pwa` 是 `test-pwa` 祖先、落后 18 个提交，这 18 个提交**没改** `useInsTaskInit.ts` / `insStore.ts`。
- 唯一差异 `bridge.ts`（feature `521-550` vs test `522-551`）仅因顶部新增 `import getTimAppId` 整体 +1 行，函数体未变。
- 结论：**同一份代码，`feature/pwa` 同样有此问题。**

## 引入者归属（别写错）

- 行级 blame（`feature/pwa`，`-L 445,485`）：这 41 行最后一次改写 = **`49521040f` 耿学岩 `<gengxueyan@presence.email>` 2026-10-28 `feat pwa 4.3`（无 MR 号）**，其 diff 是**搬位置** —— 授权回填块移到最前、`isApp()` 门挪到回填之后（先回填 `authByPlatform` 再过门），**没有新增 abnormal 写入机会**。父提交 `a54bd2486`。
- 设计层（能力来源，不等于本缺陷引入）：`d2e519857`（08-26，!960）、`ec049cebf`（08-27，!1028）、`c4e8ad0e6`（10-10，!1213），均在 `feature/pwa`（`--is-ancestor` 返回 0）。
- 文件最近改动 `2aea4c6a9`（10-30「注册流程及ins优化」）未触及该函数体。

## 取证命令（只读）

```bash
cd /Users/colna/WORK/sitin-next
git ls-remote https://gitlab.sitinai.com/frontend/sitin-next.git   # 本机需 -c http.proxy= -c https.proxy= 绕过 7890 已 closed 的代理
git rev-parse origin/release/test-pwa:packages/app-pwa/src/hooks/useInsTaskInit.ts
git rev-parse origin/feature/pwa:packages/app-pwa/src/hooks/useInsTaskInit.ts
git merge-base origin/feature/pwa origin/release/test-pwa
git rev-list --left-right --count origin/feature/pwa...origin/release/test-pwa
git show origin/feature/pwa:packages/app-pwa/src/hooks/useInsTaskInit.ts | sed -n '445,478p'
git blame origin/feature/pwa -L 445,485 -- packages/app-pwa/src/hooks/useInsTaskInit.ts
git show 49521040f -- packages/app-pwa/src/hooks/useInsTaskInit.ts
```

## 未决

- 线上卡门 A 还是门 B：需上表两条 `[INS_BRIDGE]` 日志真实样本，代码判不了。
- ref 保鲜仅到 2026-11-02 21:17（`release/test-pwa` `42be0162c` / `feature/pwa` `845a2ed7f`）。
- `bridge.ts` 的 `checkInsPageAbnormal` 二级降级在端能力侧新旧包是否存在，未验。

## 相关

- [2026-11-02 Daily](../../50-Daily/2026-11-02.md) — 本次取证工作日志 + 「切 release/test-pwa 并 reset --hard」背景。
- [SP 冷启动探测的既有设计](../../50-Daily/2026-08-06.md) — `checkInsAbnormal` 并行探 IG+SC 的设计意图（`094b8382c` / PR #824），与本条「现行是否失效」是两件事。
