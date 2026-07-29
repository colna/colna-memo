---
title: sitin-next 自拍探针「成功却总被记超时」根因 + 修法
date: 2026-07-29
tags: [sitin-next, app-pwa, sitin4.0, 探针, probe, selfie, timeout, reportTimeout, 消息金]
---

# 自拍探针任务「正常完成却总超时 + 只给文字消息奖励」

现象:女用户在 3min 内成功发出自拍并过审,却收到【消息金】文字奖励(非探针奖励),后端反馈「自拍探针总是超时」。dev 复现对:pwa `2100062929` × 男 `2100063087`(初报另一对 `2100047176`/`2100063086`)。相关后端契约见 [[sitin4.0-anomaly-probe-backend]]、[[pwa-verify-tasks]]。

## 坑

一次**成功**的自拍发送,前端会**同时**触发「成功发送」和「放弃/超时」两条路径:成功发图 + `pwa_chat_task_abandoned` + `reportTimeout(PROBE)`。后端因此把 probe 记成超时,钱却记在消息任务上。

## 根因

### 前端(真凶,可独立止血)
`PhotoTaskDrawer.done()` / `VoiceTaskDrawer.send()`:
```js
onSent?.(...);   // → handlePhotoSent → onSendImage(真发) + (旧)dismiss(report=false)
onClose();       // → dismissPhoto = dismiss(report=true) → task_abandoned + onDismiss → reportTimeout(PROBE)
```
`onSent` 之后**无条件同步** `onClose()`,而父层 `onClose=dismissPhoto=dismiss(report=true)`。于是每次点 Done 成功发送,都多打一发 report=true 的「放弃」。
- dev vconsole 铁证顺序:`ChatAuditImage passed` → `pwa_chat_task_abandoned {task_id:7419}` → `ReportTimeout {level:1(GREEN), taskType:2}` → **之后**自拍才真正 COS 上传发出(cloudCustomData `{taskIds:[7418,7419],replyKind:"probe",earnedCents:300}`)→ `pwa_task_completed {task_id:7418, reply_type:"probe_image"}`。
- 会话 status=1(GREEN);probe=7419(taskType 2),消息任务=7418(taskType 1)。

### 后端(治本,另一个仓 sitin-server,本次未改)
1. `AnomalyServiceGrpcImpl.reportTimeout` **完全忽略 `taskType`**:对 PROBE 也 `setEarningsPaused(true)`,且 `level!=RED` 一律发 `TASK_TIMEOUT_YELLOW`(GREEN 探针 → 暂停绿灯收益 + 造假黄灯事件),**不碰 probe 实体**。
2. `AnomalyTaskService.handleOutboundMessage` 不读 `cloudCustomData.replyKind`,只 `clearPending()` 清消息任务并付其 `rewardCents`(= 看到的「消息金」)。probe-only 绿灯会话则第一行 `!hasPending()` 直接 early-return。
3. `completeProbeDone` grpc 定义了但**全工作区零调用方** → probe 永不 completed。
   → 三者叠加:probe 永远收不到完成信号 = 恒超时。

## 修法

> 状态:前端修复已 **merge**(PR #758,merge commit `3c40e362`)进 `feature/sitin4.0.1`;后端 follow-up 未做。

### 前端(PR #758 已 merge,4 文件)
- `PhotoTaskDrawer.done` / `VoiceTaskDrawer.send`:`const ok = await onSent?.(...); if (ok !== false) onClose();`(`useLockFn` 防连点)。发送成功才关、失败保持打开可重试。
- `ProbeTaskDrawers.tsx`:加模块级 `const sentProbes = new Set<number>()`;`handlePhotoSent/handleVoiceSent` 改 async、`await onSendImage/Voice`、成功 `sentProbes.add(id)` 并 `return ok`,**不再自己 dismiss**;`dismiss` 的 report 块加守卫 `report && !(id && sentProbes.has(id))` —— 已发送的那次 onClose 静默关(不 `task_abandoned`、不 `onDismiss/reportTimeout`)。
- `index.tsx` `onSendImage/onSendVoice`:补 `return ok`(`sendImage/sendVoice` 本就返回 `Promise<boolean>`)。
- onSent 两处 prop 类型放宽为 `=> void | Promise<boolean | void>`。

### 后端(待对应 owner)
- `reportTimeout` 补 `taskType==PROBE` 分支:走 `handleProbeComplete(probeId,"timeout")` 清 probe,且**不**暂停绿灯收益、不发黄灯事件。
- `handleOutboundMessage` 识别 `replyKind:"probe"`(带 `taskIds`)→ `handleProbeComplete("completed")` 付 `probeRewardCents` 并清 probe(而非只清 pending)。

## 验证 / 排错要点
- app-pwa **无测试设施**(无 vitest/RTL),验证靠:`tsc --noEmit`(需先 `pnpm --filter "@heyhru/app-pwa^..." build` 构建 workspace 依赖,否则一堆 `Cannot find module '@heyhru/...'` 假象)+ dev 预览 `/dev/photo-task` + 分支自带 `console.log`(改后成功发送**不应**再出现 `ProbeTaskDrawers dismiss ... report_abandoned`)。
- 判断探针是否被误报超时:看 Loki `chat-service-api` 是否有 `AnomalyService/ReportTimeout ... taskType:TASK_TYPE_PROBE`;`level` 为 GREEN 更说明是绿灯误伤。
