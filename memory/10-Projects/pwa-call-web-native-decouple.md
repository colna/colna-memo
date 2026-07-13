---
title: PWA 通话模块 web/native 解耦方案
date: 2026-07-13
tags: [sitin-next, app-pwa, call, refactor, architecture]
---

# PWA 通话模块 web/native 解耦方案

> 目标:让 web 与 native 两套通话逻辑解耦 —— 能单独改一边不碰/不破坏另一边,且计费等业务规则单点维护。
> 代码位置:`sitin-next/packages/app-pwa/src/`(`services/webCallManager.tsx` `services/nativeCallManager.tsx` `hooks/useWebCall.tsx` `hooks/useNativeCall.tsx` `hooks/useCall.tsx`)。
> 相关:[[sitin-next]]、[[pwa-female-wallet-billing]]

## 现状诊断(通读四文件后)

规模:webCallManager 989 / nativeCallManager 420 / useWebCall 925 / useNativeCall 536,共约 4160 行。三套栈(web/native/mock)在 `useCall` 里**无条件同时挂载**,靠内部 `isApp()` 守卫决定注册。

**分界线画错了位置**:web/native 的差异本应只在「传输层」(TUICallEngine vs native bridge),但计费/审核/结算这些平台无关逻辑被沿 web/native 切成了两份。

真正的乱:
1. **计费/结算重复**:`getCurrentOrderEarned`(useWebCall:305 vs useNativeCall:152)**逐字节相同**;`getCallRecordParams` ~95% 相同;按分钟违规扣款(`totalDeductedMinutesRef`/`violationDeductedRef` + 「≥3 次违规 → pwaEarnDeduction → 累计扣 1 分钟」)两份;结束结算编排(奖励/太短/收益失败/记录弹窗)两份。→ 改扣款规则要改两处,是「靠拷贝耦合」的假独立。
2. **imListener 重复**:两 manager 里 CallOrder/AVCall 解析块逐字节相同(web:232-256 vs native:164-184);`getCallStoreActions`/`setCallStateToStore`/单例模板也都是两份。
3. **命名/结构分叉**:`cloesConnect`(web,拼写错)vs `closeConnect`(native);handler 三套风格(`handleCallBegin` vs `handleCallBeginFromNative` vs `handleNotifyCallFinished`);`connectSession` 初值都是假的全 0 对象而非 `null`,且字段还不一样(web 多 callerId/calleeId/source/hasReportedCallEnd)。
4. **web hook 里夹着 native 代码**:useWebCall `onVoiceViolationToast`(196)还 `if (isApp())` 走原生弹窗 —— 分层不彻底。
5. **疑似真 bug**:webCallManager `handleCallError`(870-900)**先** `cloesConnect()` 置 null,**后**在 `if(this.connectSession)` 里读它发 bpTrack → 那段埋点是死代码。

## 决策:走「抽象解耦(B)」而非「隔离解耦(A)」

- A 靠隔离(各自独立、允许重复):改一边不碰另一边,但重复永久存在、改规则要改两处 → 假独立。
- **B 靠抽象(端口-适配器 / strategy)**:抽平台无关公共域 + web/native 只做薄适配器;两边互不引用、都只依赖抽象契约。改 web 不破 native,计费单点维护。← **采用 B**。

## 目标架构

```
平台适配器(只有这层分 web/native,彼此不互相引用):
  WebCallManager   (TUICallEngine 适配) ─┐
  NativeCallManager(bridge 适配) ────────┴─► 归一化 CallEvents(eventBus)+ 归一化 ConnectSession
                        │
                        ▼
平台无关公共域(只写一份):
  useCallSettlement   计费/收益/违规扣款/记录弹窗
  useCallModeration   语音/人脸/图谱/AI 转写 的 start-stop
  会话状态机 + 结束结算编排
```

三原则:
1. 归一化事件契约:两 manager 发同一套抽象事件(`call:invited/begin/ended/canceled`)+ 同一 `ConnectSession`,不再 `CALL_BEGIN` vs `NATIVE_CALL_BEGIN`。
2. 公共域只依赖契约、不知道 web/native。
3. 平台判断只在入口一处(按 `isApp()` 选适配器),其它地方不再写 `isApp()`。

## 进度

- **Step 1 已完成** → PR #593(base `feature/sitin4.0`,分支 `personal/zz/pwa-call-decouple`,2026-07-13)。做了:CallControllers 按平台拆挂载 + useCall 不再无条件挂三 hook;useWebCall 清 isApp 分支变纯 web;`entry` 枚举 → `startCall(..., { maxDurationMs })`,`VIDEO_TIPS_CALL_LIMIT_MS` 移到 types/call.ts。tsc/lint 过,真机未验。
- Step 2–5 待后续独立 PR。

## 落地路径(小步、每步可验证,不 big-bang)

- **Step 1(最先做,低风险)✅ 已完成(PR #593)** 只挂载当前平台的栈。现 `useCall` 无条件挂 web+native+mock;改成组件拆分 `{isApp() ? <NativeCallController/> : <WebCallController/>}`,每个 controller 内部调各自 hook(避开顶层条件调用 hook 的规则问题)。顺手清掉 useWebCall 里残留的 `isApp()` 原生分支。
  - **同步做:去掉 `entry` 枚举,改成 `startCall` 显式 options。** 现状 `CallEntry = "video_tips" | "debug"` 只用于「video_tips → 1 分钟限时」这一个中心 switch(`startCallLimit`),`"debug"` 无行为;`entry` **未用于埋点**(来源统计走 `session.source`),删枚举安全。改为调用方直接传行为参数,传对象不传裸值:`startCall(..., { maxDurationMs?: number })`,值用共享常量 `maxDurationMs: VIDEO_TIPS_CALL_LIMIT_MS`。放层级仍遵守解耦:**传输适配器签名保持 `startCall(userID, type, orderId)` 不接 options**;options 属于域层「发起意图」,写进归一化 ConnectSession,由共享 `useCallLimit` 消费 —— 顺带补上「native 侧当前无限时」的 gap(同一策略跑两端)。
- **Step 2(去重收益最大)** 抽公共域 hook:`getCurrentOrderEarned`/`getCallRecordParams`/违规扣款/结算弹窗 → `useCallSettlement`;审核运行时 → `useCallModeration`。两 controller 复用。
- **Step 3** 抽公共 imListener:CallOrder/AVCall 解析 → `attachCallImListener(getSession)`,两 manager 复用。
- **Step 4** 归一化事件 + ConnectSession + 命名统一(`cloesConnect`→`closeConnect`、handler 命名对齐、`connectSession` 初值改 `null`)。改动面大、结算时序敏感,单独一轮。
- **Step 5(可选终态)** 事件归一后,公共域由一个平台无关 `useCallSession` 编排,两 controller 退化成纯 wiring。

顺手可做:修 Step 5 无关的 `handleCallError` 死代码 bug(Step 4 一起或单独)。

## 建议起步

从 **Step 1 + Step 2** 开始即可解决 ~80% 的乱(彻底不再交叉挂载 + 消灭最危险的计费重复),风险低。别一上来做 Step 4/5 事件归一(时序敏感)。

## 背景约束(勿丢)

- orderId 现有 3 条送达路径(有意冗余兜底):TUICall 邀请 `userData`(最可靠)+ CallOrder TIM 消息 + 主叫本地 `connectSession.orderId`。解耦时保留,别削路径。
- 女方主叫建单:`maleUserId=对方, femaleUserId=自己`;安卓男主叫相反。共享逻辑已抽到 `services/outgoingCallOrder.ts`(本轮已完成的一步,是 B 方案的样板)。
- 分支 `personal/zz/pwa-web-call-order`(PR #591)已含:先建单 → 补发 CallOrder TIM → 修 startRemoteView 5000 竞态 → 主动拨打按平台拆 → userData 带 orderId。本解耦方案是其后续。
