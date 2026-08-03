---
title: PWA 视频通话涉及的端能力（native bridge）+ 违规弹窗链路
date: 2026-08-03
tags: [sitin-next, app-pwa, call, bridge, native, moderation, violation, reference]
---

# PWA 视频通话端能力（native bridge）+ 违规弹窗链路

> 代码位置：`sitin-next/packages/app-pwa/src/`。所有 `invokeBridge("xxx")` 即一个端能力（定义在 `utils/bridge.ts`）。
> 仅在 `isApp()`（APK WebView，`window.pwaBridge` 存在，bridge.ts:622）环境生效；web/H5 不走原生桥。
> 相关：[[pwa-call-web-native-decouple]]、[[pwa-female-wallet-billing]]

## 一、JS → 端（invokeBridge 主动调用）

| JS 封装函数 | 原生方法名 | 位置 | 作用 |
|---|---|---|---|
| `startAVCall` | `startAVCall` | bridge.ts:522 | 发起原生外呼（引擎+UI 在 native），传 userId/type/orderId |
| `notifyCallType` | `notifyCallType` | bridge.ts:813 | 通知端本次是否免费通话 |
| `registerSensor` | `registerSensor` | bridge.ts:558 | 启动陀螺仪，判女方是否手持（违规证据），默认 stillTimeout=60000 |
| `unregisterSensor` | `unregisterSensor` | bridge.ts:578 | 停止陀螺仪，与 register 成对 |
| `snapshotSelfVideoPicture` | `snapshotSelfVideoPicture` | bridge.ts:742 | 截自拍帧为 Base64，供图普内容审核（native 无 video 元素时兜底取帧） |
| **`showViolationToast`** | **`coiceViolationToast`** | bridge.ts:759 | **违规弹窗（原生）**，仅 native 侧调用。⚠️ 原生方法名拼写就是 `coiceViolationToast` |
| `redirectToFaceIdPage` | `redirectToFaceIdPage` | bridge.ts:836 | 原生人脸活体验证（准入/头像认证） |
| `hangupAVCall` | `hangupAVCall` | bridge.ts:609 | 挂断原生通话（限时挂断；端上暂未实现，返回 404 拒绝） |
| `setNativeInterval` / `clearNativeInterval` | 同名 | bridge.ts:786 / :800 | 原生定时器（通用） |

## 二、端 → JS（registerBridgeHandler 原生回调）

| 回调名 | 作用 |
|---|---|
| `onHoldingStateChanged` | 陀螺仪手持状态变更 → 更新 `isHoldingRef`（判罚），useNativeCall.tsx:417-432 |
| `getViolationCount` | 端查询/同步违规次数 |
| `videoConvertVoiceCallback` | native 语音转写文本 → `NATIVE_VOICE_TRANSCRIPTION`（nativeCallManager.tsx:568）→ 喂语音审核 |

## 三、按用途归组

- **通话起止**：`startAVCall`、`hangupAVCall`、`notifyCallType`
- **违规弹窗**：`showViolationToast`（=`coiceViolationToast`）← 核心
- **审核证据采集**：`registerSensor`/`unregisterSensor`（陀螺仪）、`snapshotSelfVideoPicture`（图普取帧）、`videoConvertVoiceCallback`（语音转写）、`getViolationCount`、`onHoldingStateChanged`
- **人脸真人校验**：`redirectToFaceIdPage`
- **通用**：`setNativeInterval`/`clearNativeInterval`

## 四、违规弹窗链路（重点）

平台分叉在挂载边界 `components/CallControllers.tsx:35`（`IS_NATIVE = isApp()`），按平台只挂 `useNativeCall` 或 `useWebCall`；`showViolationToast` 本身不含分叉。

| 环境 | 弹窗方式 | 调用点 |
|---|---|---|
| **Native(APK)** | 原生弹窗 = `coiceViolationToast` 端能力 | `useNativeCall.tsx:115`（语音 onVoiceViolationToast）/ `:121`（图普 onTupuViolationToast）→ `showViolationToast` |
| **Web(H5)** | React 弹窗 `showViolationLimitModal`，**不走 bridge** | `useWebCall.tsx:179`（语音）/ `:148`（图普） |

违规产生点（平台无关，在审核 hook 内）：
- 语音：`useVoiceModeration.recordViolation`（:308）→ 达标 `onVoiceViolationToast`（:350）
- 图普：`useVideoScreenshotViolation.handleViolationEvent`（:181）→ `onTupuViolationToast`（:242）

### 三类审核

| 类别 | 文件 | 触发 |
|---|---|---|
| 语音 voice | `hooks/useVoiceModeration.ts` | 18s 静音检测 + 本地正则（金钱 MONEY_PATTERNS / 声称 AI 的 AI_PATTERNS）判转写文本 |
| 图普/截图 tupu | `hooks/useVideoScreenshotViolation/index.ts` | 每分钟随机 2 次截图（10-20s、35-50s）调图普科技查色情/性感 |
| 人脸 face | `hooks/useFaceDetect/`（web）+ 陀螺仪 `registerSensor`（native）+ native faceRate | 露脸率 + 是否手持，结束时综合判罚 |
| AI 转写 | `hooks/useAITranscription.ts` / native `videoConvertVoiceCallback` | 给 voice 审核提供文本 |

### 阈值

- 三类审核共享 `sharedViolationCountRef`（= `useCallViolationDeduction.violationCountRef`），每次违规 +1。
- 弹窗上限 `MAX_VIOLATION_COUNT = 3`（useVoiceModeration.ts:24）：第 1/2/3 次弹窗展示收益 100%/50%/0$；`> 3` 次**不再弹**且 `disableEarning = true`。
- 静音：前 15s 内 5s 无声弹 NoVoice，15s 后 15s 无声弹；18s 静音记一次违规（SILENT_TIMEOUT=18000）。
- 按分钟违规扣款：`useCallViolationDeduction.ts`，`VIOLATION_DEDUCT_THRESHOLD = 3`——每分钟检查，本分钟违规 ≥3 且未扣过且 orderId 有效 → `pwaEarnDeduction(orderId, price)` 扣一分钟收益，累加 `deductedMinutesRef`（= `totalDeductedMinutesRef`），供 `useCallSettlement` 结算读取。
- 图普特例：检测到"男性"（TUPU_ILLEGAL_MAP[8]）不弹窗不计数仅上报；上报 `needDeduction = currentCount >= 3`；5 分钟后只检非色情违规。

### 弹窗传参（PopupTextConfig）

```
{
  rejectedCount, maxReject: 3,
  timesTip, closeText, answerText, secondary,
  earningOneVoice: "100%", earningTwoVoice: "50%", earningThreeVoice: "0$",
  warning,                 // getViolationTip 按违规类型给
  violationType?: "inactivity" | "explicit_profit_motive" | "AI_interaction" | "other_violations"
}
```
生成点：语音 `generatePopupConfig`（useVoiceModeration.ts:205）、图普 `nativePopupText`（useVideoScreenshotViolation/index.ts:223-237）。

## 五、生命周期时序（native）

```
发起   startAVCall(:522)  notifyCallType(:813, if isApp)
接通   registerSensor(:558)  ← handleNativeCallBegin useNativeCall.tsx:171/197
       ├ 启动 语音/图普/AI转写 审核 + 按分钟扣款 startMinuteCheck
       └ 图普取帧 snapshotSelfVideoPicture(:742)
违规   showViolationToast → coiceViolationToast(:759)【仅 native；web 走 showViolationLimitModal】
人脸   通话中 陀螺仪 isHolding + faceRate 综合判罚(useNativeCall:338，!isHolding && faceRate<0.5 关分发)
       准入 redirectToFaceIdPage(:836, if isApp)
结束   stopVoice/Tupu 检测 → unregisterSensor(:578)  → (限时) hangupAVCall(:609，端上未实现)
```
