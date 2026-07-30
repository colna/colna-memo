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

---

## 探针 reward 金额取错:探针 reward 只有一条且 type=0，别按媒体类型查(PR #780)

来源:2026-07-29,QA 报「探真自拍发出后图片气泡金额显示 +0.8¢,应为探针奖励 ~20¢」。

### 根因(真机 convState JSON 实测)

探针 task 的 `reward` **只有一条、且 `type` 是 `0`(REWARD_TYPE_UNSPECIFIED)**:
```json
{ "taskType": 2, "probeType": 4,   // PROBE + SELFIE
  "reward": [ { "type": 0, "cents": 2000 } ] }   // 探针:单条、type=0、20¢
```
而消息(PENDING)任务是**带 type 的三条**:
```json
"reward": [ {type:1(TEXT),cents:160}, {type:2(VOICE),cents:240,critHit}, {type:3(IMAGE),cents:160} ]
```

`Chat/index.tsx` 的 `getRewardCents` probe 分支原来走 `getRewardCentsByType(probeReward, "image"/"voice")` —— 该函数按 `REWARD_TYPE_IMAGE(3)`/`VOICE(2)` 匹配、兜底 `TEXT(1)`。探针 reward 是 `type:0` → **三种都匹配不到 → 返 0 → `|| normalReward()` 回退到消息任务的对应奖励**(如 image 的 160/1.6¢ 或 80/0.8¢)。**photo 和 voice 同病**(voice 气泡其实也错,只是探真抽屉用 `reward[0].cents` 显示对、易被忽略)。

### 修法(前端,PR #780)

probe 分支改为**与探真抽屉同源、直接取 `probeTask.reward?.[0]?.cents`(不看 type)**:
```js
const probeCents = getProbeTask(cv)?.reward?.[0]?.cents;
if (probeCents) return raw ? probeCents : toRewardCents(probeCents);
return normalReward();
```
同时修好**图片气泡显示金额** + **cloudCustomData 的 earnedCents**。videoTips 分支 reward 是按 `VIDEO` 类型正确匹配的,不动。

### ⭐ 教训:reward 结构类问题别凭「行为」推断 type,直接看响应 JSON

我最初从「image 查询回退、voice 没报错」**推断**探针 reward type=VOICE —— 被真机 JSON 直接推翻(**根本没 type / type=0**)。凡是「按类型/字段取值取错」的 bug,先抓一份**真实响应 JSON** 看字段,别用外部行为反推内部结构(和 §mobile-keyboard §13「调试指标本身会说谎」同源)。

### 后端 follow-up(前端已不依赖)
探针 reward 的 `type` 应按探针类型给(selfie→IMAGE(3) / voice→VOICE(2)),而不是 `0`。前端改成读 `reward[0]` 后不 block 后端,但语义上后端补 type 更干净。
