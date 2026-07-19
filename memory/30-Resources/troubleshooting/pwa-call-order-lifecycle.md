---
title: 通话订单（call order）生命周期 —— PWA 女端 / Kira 男端 / GraceChat-Earn 三端分工
date: 2026-07-17
tags: [troubleshooting, sitin-next, app-pwa, call-order, finishCallOrder, tuicall, billing, android]
---

# 通话订单 call order 三端分工与踩坑

sitin4.0 视频通话计费的真相。跨 sitin-next(PWA) + Kira-Android(男) + GraceChat-Earn-Android(女壳) 三仓逆向得来。

## 谁建单、谁加钱、谁关单

| 端 | 包名 | createCallOrder | addMoney(通话中每60s) | finishCallOrder(结束) |
|---|---|---|---|---|
| **Kira**（男，真消费端） | `com.meraki.kira` | ✅ 仅自己主叫 | ✅ **无角色门禁** | ✅ **无角色门禁**，只在 `onCallEnd` |
| **GraceChat-Earn**（女端 App，PWA 的 native 宿主） | `com.harbor` | ❌ | ❌ | ❌ |
| **PWA**（女，web + App 内 WebView） | app-pwa | ✅ 女方主叫（`placeOutgoingOrder`） | ❌ | ✅ **本次（2026-07-17）才补上**，之前一处没有 |

- **男方主叫**：Kira `goStartVideo2` 建单 → `mVideoCallOrderId` → 通话中每 60s `addMoney(CALL_ORDER_REAL_TIME)` 给女方实时加钱 → `onCallEnd` `finishCallOrder(CALL_ORDER_FINISHED)` 只关单（钱实时加过了）。
- **女方主叫**（web/native）：PWA 建单 → orderId 塞 TUICall 邀请的 `userData` + 推一条 CallOrder TIM 消息给对端。**但对端 Kira 作为被叫两条都不消费**：`CallEngineObserver.kt` 的 `onCallReceived` 不读 userData；CallOrder TIM 消息被 `RMChat.kt:459` 当系统消息丢弃。Kira 的 `mVideoCallOrderId` 只在自己主叫时赋值，被叫时是 0。**→ 女方主叫的订单三端没人关，建完就悬空。** 这是 2026-07-17 那批 PR 补的洞。

## balanceType 三个值（proto `user_api.proto` CallOrderBalanceType）

| 值 | 语义 |
|---|---|
| `CALL_ORDER_UNKNOWN=0` | 老逻辑：**关单并加钱** |
| `CALL_ORDER_REAL_TIME=1` | 通话中实时加钱（男方 addMoney 用） |
| `CALL_ORDER_FINISHED=2` | 只关单、**不加钱**（实时已加过） |

**⭐ 传错就是真金白银**。女方主叫最终定 `CALL_ORDER_UNKNOWN`（关单并加钱）—— 因为客户端没人给女方主叫的通话调 addMoney，这一单的钱只能靠 finishCallOrder 结。决策依据是「猜错的代价方向」：传 UNKNOWN 若后端也加 = 重复打钱不可逆；传 FINISHED 若没人加 = 女方白打（等于现状、可逆）。**先按代价方向选，业务确认后再定。**

## reasonType（CallOrderFinishReasonType）是男方主叫视角命名的

`FEMALE_CALL_REJECT` / `FEMALE_BUSY` / `FEMALE_NO_RESPONSE`（女方作为被叫）、`MALE_CANCEL_*` / `NORMAL_MALE_HANG_UP`（男方本端）。**没有女方主叫被拒/忙线/未接的对称值**。女方主叫时除「我方取消」用 `INIT_FEMALE_CANCEL`，其余只能 `NORMAL`；要精确归因得扩枚举（后端配合）。

## 单位陷阱（差点 100 倍）

- 视频小费卡片的 `centsPerMinute`（后端随卡片下发）是**分**。
- `ConnectSession.price` / `FinishCallOrderRequest.price` 是**美元**。铁证：mock 的 price 直进 `CallRecordModal.tsx:141` 的 `earned.toFixed(2)`。
- → `VideoTipsMsgBubble` 传 `centsPerMinute / 100`。

## PWA 侧实现要点

- **web/native 不能收口到一个 manager**：结束事件不相交（`CALL_ENDED` vs `NATIVE_CALL_END`），`CallControllers.tsx` 二选一挂载。但**结钱口径必须一致** → 抽 `utils/callerPayout.ts` 共用（web `webCallManager` + native `nativeCallManager` 都调）。
- **⭐ 关单只认自己建的单**：存 `outgoingOrderId`（startCall 时），**别读 `session.orderId`** —— 后者被 `callImListener.ts:26` 在被叫收 CallOrder 消息时原地改写，且 `source` 是粘的（`closeConnect` 正常结束流程不调用），叠加会「上一通主叫 source + 这一通被叫 orderId」把对端的单当自己的关掉。
- **别抄 Android 的 `duration < 2` 判秒挂** —— 那是 Kira 把「女方拒接」误报成「男方秒挂」的根源（callDuration 用 billingCounter，未接通恒 0）。PWA 用 `callBeginTime === 0` 判「从未接通」。
- **视频小费结钱规则**：≥30s 传卡片 price，<30s 传 0。**只看时长不看违规** —— 违规本来按分钟扣（`pwaEarnDeduction`），再 gate 一次是双罚。

## ⭐⭐ 最大的坑：别在 SDK 全局 ERROR 出口关单

`webCallManager.handleCallError` 绑的是 `TUICallEvent.ERROR`，那是 **SDK 全局错误出口，不等于通话终止**。绑远端视频的瞬时错误也走它、且必然遇到：

- `5100` cannot startRemoteVideo because remote user does not publishing stream —— 对方还没推流，本就有 4 次重试
- `5000` 'view' is not found —— 通话页懒挂载，`#remote-video` 还没进 DOM

**这两个错误发生时通话还在继续。** 在这里关单 → 通话中途抖一下就提前结钱、按错时长结（视频小费 duration<30 → price 0 白打）。

**关单只挂在「通话真结束」的出口**：`handleOutgoingFailure`（接通前失败）/ `handleCallEnd`（接通后）。native 对应 `handleNotifyCallFinished` / `handleCallCancelFromNative`。

> **根因是个通用错误模式**：`handleCallError` 里既有代码在 `closeConnect()`，我把它当成「通话完了」的佐证 —— 而那本身就是既有 bug（瞬时错误不该清 session）。**从可疑的既有代码推断语义 = 把别人的 bug 当规格。** 同源教训见 [[sitin-next-testing-and-merge]] 里 dist 那条「别从少量既有样本推断约定」。

## Android（Kira）既有 bug（PWA 改不了，需单独跟进）

- `onCallEnd` 不区分 `CallEndReason`，未接通场景（billingCounter=0）一律误报 `NORMAL_MALE_HANG_UP`。
- `finishCallOrder` 无重试、无持久化（`mVideoCallOrderId` 是内存 Int，进程杀就丢）。
- `resetBillingStatus` 不清 `mVideoCallOrderId` → 残留 orderId 可能被下一通的 addMoney 用。
- 被叫时计费循环照跑、用 `orderId=0` 发 addMoney/finishCallOrder。
- `MALE_CANCEL_WITHIN_5S` / `MALE_CANCEL_TIMEOUT_5S` proto 有定义、Kotlin 零引用 —— 那两个值得去 luma/romi 仓库找（Kira ≠ luma/romi）。

## native（GraceChat-Earn）既有缺口

- `notifyCallFinished` 回传**没有 orderId**（`SendCallStatusUtil.kt:69` 只给 callType/duration/roomID/userId/faceRate/hangupId），但 PWA `nativeCallManager` 却在解构它 → 永远 undefined。orderId 只能 JS 侧自己存。
- native **完全没有 60s 限时挂断**（web 有 `startCallLimit`）→ 视频小费在 App 内不会自动挂断。
- `callCancelFromNative` payload 只有 callType，分不出「我方取消」和「对方拒接」。
- native 无 `handleCallError` 对等回调 → 通话出错关不了单。

## ⭐ 挂载层决定生命周期：限时/发奖编排必须挂常驻层，不能挂 chat 页

`CallControllers.tsx:40` 按 `IS_NATIVE` 二选一挂 `WebCallController`/`NativeCallController`，还常驻挂 `MockCallController`。**它是 `<AppRouter/>` 的 sibling，不随路由变** → 挂在这里的 hook 通话全程不卸载。

反例（我 refactor #650 踩的坑）：把 videoTips 的限时/发奖编排 `useVideoTipsCall` 挂进 chat 页（`ActiveChat` 内）。**接通后 `useWebCall` 会 `navigate("/video-call")`，而 `/video-call` 是 `/chat` 的对等 Route，会卸载整个 `/chat` 子树** → hook 随之卸载：

- 60s 限时 timer 在接通瞬间被 effect cleanup 清 → **web 仍不在 1min 自动挂**；
- 结束时 onEnd 已不在 → **奖励丢**。

**修法**：hook 改自包含、无 chat 依赖、无参数，随 `CallControllers` 常驻在 App 根（新增 `VideoTipsCallController`，和 `MockCallController` 同级）。自记接通时长判发奖、金额取发起时 `centsPerMinute`、金币动画走全局 `modalStore`（不经 chat 的 mountedRef gate）。

> 规律：**凡「跨接通、跨导航要活着」的编排（限时、发奖、订单收尾），必须挂常驻层，不能挂随路由卸载的页面。** 底层通话（callService/CallManager/useWebCall）只管生命周期+建单+发 TIM+按传入 price 结钱，不认「videoTips/限时」。

## ⭐ native 结束守卫 isEnded 两个入口都要复位（P1，2026-07-18 修）

web 的 `beginCallLifecycle` 在**被叫和主叫两个入口都复位** `isEnded`/`isUserHangup`；native 的 `useNativeCall` 原本只在**被叫入口**（`handleNativeCallInvited`）复位，**主叫入口**（`handleOutgoingStart`）漏了。

后果：第 2 通及以后的 native 主叫，上一通结束置的 `isEnded=true` 残留 → `handleNativeCallEnd` 被 `if(isEnded.current) return` 直接吞 → **订单结钱了但结算/发奖/导航回退全丢**。

修法：`handleOutgoingStart` 拿到 targetUserId 后补 `isEnded.current = false`。教训：**web/native 两套入口，对称的状态复位 web 抽成一处、native 是两处手写 → 天然容易只改一处，改 native 生命周期时逐一核对被叫/主叫两入口。**

## 相关

- 现有主动外呼实现见 [[tuicallkit-web-ui]]、[[../../50-Daily/2026-07-07]]（PR #540）、[[../../50-Daily/2026-07-12]]（orderId 走 userData）。
- 女方钱包/计费口径 [[../pwa-female-wallet-billing]]、reasonType 由男方 App 上报 [[../grafana-loki-query-manual]] §6。
