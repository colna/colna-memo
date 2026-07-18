---
title: 关闭的 app-pwa 老 PR 存档（方便以后找回）
date: 2026-07-17
tags: [archive, sitin-next, app-pwa, pr, closed]
---

# 关闭的 app-pwa 老 PR 存档

2026-07-17 清理张峥名下 5 个长期挂着的 open PR。它们 base 都是**旧时代分支**
（`feature/pwa` / `release/online-pwa` / `feat/sitin/gxy`），不是当前主线 `feature/sitin4.0`，
`mergeable` 长期 UNKNOWN。**关闭不删分支** —— 分支和 commit 都还在远端，需要时可从下面的分支名捞回或重开 PR。

> 捞回方式：`git fetch origin <headRefName>` → cherry-pick 想要的 commit，或 `gh pr reopen <号>`（分支未删时可重开）。

## 值得留意（有实际修复，可能要重新应用到 sitin4.0）

### #451 · mock 通话人脸检测 faceRate 恒为 0
- 分支 `fix/pwa-mock-facedetect-race` → `feature/pwa`（2026-06-23，1 commit）
- **根因**：人脸模型加载(~40s) 长于 mock 通话窗口(35s)，start/stop 竞态 → 加载完成后创建出指向已 dispose 状态的「僵尸检测循环」→ faceRate 永远 0 且后续通话无法恢复。
- **改法**：`useFaceDetect` 模型改**进程级单例**跨通话复用（不再每通 dispose）+ runId 竞态保护；`FaceDetectService.detectForVideo` 用单调递增时间戳避免新流 currentTime 回退抛错；`MockCallView` 进页面即预热模型。
- 文件：`hooks/useFaceDetect/{FaceDetectService.ts,index.ts}`、`pages/MockCall/MockCallView.tsx`
- ⚠️ **未确认这个 bug 在 sitin4.0 是否仍在**。若 mock faceRate 又出问题，先看这个 PR 的分支。

### #590 · 会话列表缓存正确性与减量修复
- 分支 `fix/pwa/chat-list-cache` → `feat/sitin/gxy`（2026-07-11，2 commit，tsc/lint/circular 全过、未跑真机）
- **背景**：某次重构删了主动全量拉取（旧 `buildList`），列表变纯事件驱动，加剧「事件不可靠 → 缓存残留服务端已删数据」。
- **改法**：① 红/黄会话保护 `withProtectedConvs`（全量替换前把 `tasks?.length>0` 但被 isPaid 过滤的会话并回）；② 主动减量 `rebuildList`+`resync`（`NET_STATE_CHANGE`/`visibilitychange` 时 `getConversationList` 全量替换，清另一台设备删的会话）；③ 账号隔离（generation 计数器 + 快照带 `ownerUserId`）；④ persist 加 version+migrate、localStorage 1.5s 节流。
- 文件：`services/chatConvManager.ts`、`stores/chatConvStore.ts`、`docs/会话列表缓存-技术设计.md`
- ⚠️ base 是 `feat/sitin/gxy`（xueyangeng 的分支），不是主线。若 sitin4.0 会话列表有缓存残留问题，捞这个分支的思路。

## 历史 / 实验 / 已过时（基本不用捞）

### #164 · Personal/pwa/fix mock（2026-05-15）
- 分支 `personal/pwa/fix-mock` → `feature/pwa`，**27 个 commit 大杂烩**，body 空。
- 混了：go live modal、pwa lark bot、ai 语音检测/翻译/正则、mock 中断 live、native violation、语音审核去重（native ASR interim）、Connecting/Ringing 拆分、结算弹窗用 session snapshot 显示正确金额、cashout model 冲突……外加一堆 `.claude`/`.serena`/`.agents` chore。
- **判断**：内容早已零散合入或被 sitin4.0 重写，杂到没法整体捞。真要找某个具体修复（如「结算弹窗金额 snapshot」）按关键词去分支里翻单个 commit。

### #147 · Fix/pre commit（2026-05-13）
- 分支 `fix/pre-commit` → `feature/pwa`，5 个 commit 全是「commit 测试」「--circular」，body「修复 commit lint 文件过多问题」。
- **判断**：纯 pre-commit hook 调试的临时分支，无业务价值，直接废。

### #55 · Personal/pwa/fix mock zz（2026-04-27，最老）
- 分支 `personal/pwa/fix-mock-zz` → `release/online-pwa`，4 commit。
- 内容：线上 mock 视频计费修（打了一通 mock 给 1.5，中间来真人视频没到 30s 没给钱，又一通 mock 又给 1.5）、第二阶段需完成两个测试视频才升级视频小利女、web-vitals、挂断后无声音检测弹窗持续显示。
- **判断**：base 是 3.0 时代的 `release/online-pwa`，计费逻辑 sitin4.0 已重写（见 [[../30-Resources/troubleshooting/pwa-call-order-lifecycle]]）。历史存档，不捞。

## 相关
- 当前主线通话计费真源 [[../30-Resources/troubleshooting/pwa-call-order-lifecycle]]
