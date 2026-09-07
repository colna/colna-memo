---
title: koda 接入恋爱铃(Love Ring / 假来电)全套
date: 2026-09-07
tags: [project, koda, sitin-rn, love-ring, fake-call, call, growth]
---

# koda 接入恋爱铃(Love Ring)全套

**范围(用户 2026-09-07 定)**:**全做** —— 恋爱铃本体 + 两个孪生子系统(全屏来电邀请 full-screen-invite + PWA 回播 pwa-video-recall)。照 iris 移植到 koda。
**方式**:先 file-for-file 落在 koda 内(不抽共享包),三套稳定后再评估抽 headless `@heyhru/business-fake-call`。
**模式**:规范模式,逐 Phase→Task,每步 biome/typecheck/boundaries/test 过,提交/push 前问用户。

## iris 三子系统(互斥)
| 子系统 | service | 组件 | S2C | 握手 |
|---|---|---|---|---|
| 恋爱铃本体 | `services/call/love-ring.ts` | `components/call/love-ring-banner.tsx` | StartLoveRing/LoveRingPush/ReportLoveRing | 双向 |
| 全屏来电邀请 | `services/call/full-screen-invite.ts` | `components/call/full-screen-invite.tsx` | S2C_FullScreenVideoPush | 被动 |
| PWA 回播 | `services/call/pwa-video-recall.ts` | `components/call/pwa-recall-invite.tsx` | PwaVideoRecall/ReportPwaVideoRecall/CancelPwaVideoRecall | 半(HTTP重排队) |
三者 `isXShowing()` 互斥;`FakeCallHost`(tab 根)统一 arm+挂载。

## 恋爱铃握手序列(核心)
1. arm:Alive `Connected` → `start()`(hasTrigger 只一次)→ addObservers → `triggerLoveRing()`。
2. **C2S_StartLoveRing{loveRingType=MATCH_HOME}** 上行(不发这个,后端永不推 push)。
3. S2C_StartLoveRing = ack。
4. **S2C_LoveRingPush{user:UserInfo}** → `showPopup(user)`。
5. 门禁:AB `g_fake_call_switch`(默ON)→ `!isDtcPurchasing` → `AppState active` → `!任一弹窗showing` → `!isCallingNow`。失败 → reportShowFailed(DISPLAY_FAILED)。
6. banner mount → reportShowSuccess → C2S_ReportLoveRing(SUCCESS) + reportUserEvent(LOVE_BELL_EXPOSURE)。
7. accept → placeCall(...,CallSource.FakeCall);decline → CANCEL + 掉未接来电气泡;10s 超时 → AUTO_DISMISS。
8. 通话结局(call-session,gated callSource===FakeCall):onCallEnd→ACCEPT_CONNECTED_EXIT / onCallCancelled→ACCEPT_NO_ANSWER / onUserReject→ACCEPT_REJECTED / noResp·busy·dialErr→ACCEPT_NOT_CONNECTED。
9. 频控:`lastActionTime` 持久化;前台回来距上次>120s 才重触发(从未发过则跳过)。
未接来电气泡:仅对已是会话好友(chat conversations 命中 peerId),走 `sendFakeCallRecord`→HTTP `SendMessageFromBotRequest`(对方 bot 账号发,CUSTOM_DESC.PHONE_CALL)。

## koda 已可复用
Alive 长连接(`services/alive/*`,registry 已含恋爱铃三标签)、LoveRing proto(`business-pwa-proto` AiTcp.* 共享)、埋点(`LumiEvent=createAnalyticsEvents("koda")` → 自动 koda 前缀 `koda_fake_call_*`,天然不串包)、通话 A 阶段(call-session/use-call/call-state/call-precheck/call-billing/call.ts)、`isDtcPurchasing`、`isCallingNow`、`match-push.ts`(订阅S2C→store→组件范式样板)、`lib/storage.ts`、chat/session/entitlements store、`theme/tokens.ts`(暖橙奶油)。

## koda 特有适配(务必)
- UI 用 koda token 重画,**不照搬 iris** 视觉。
- 埋点走 `LumiEvent.*`(koda 前缀),**不硬编码** `fake_call_*` 字面量。
- **SecureStore key**:冒号非法且静默失败 → iris `lovering_last:${uid}` 改 `koda_lovering_last_send_${uid}`([A-Za-z0-9._-])。
- AB key 用 koda 命名常量包装(§4.2 只约束客户端源码;后端 key 归属待确认)。
- `useSessionStore` 替 `useAuthStore`;`useKodaChatStore` 替 `useChatStore`。
- 无 `mapUserInfoToCandidate` → banner 直读 UserInfo 字段。
- 顺手清 `ab-test.ts` 的 lumi 命名残留(getLumiAbManagerState 等,技术债)。

## 后端硬前提(用户说"客户端先做")
koda 独立后端。需:①Alive 收 C2S_StartLoveRing、推 **S2C_LoveRingPush{user}**、收 C2S_ReportLoveRing/ReportUserEvent ②HTTP `SendMessageFromBotRequest`(未接来电气泡)③AB 开关配置 ④候选池策略 ⑤孪生子系统各自的 S2C(FullScreenVideoPush / PwaVideoRecall 三件 + VideoCallQueueApply)。未就绪前只能本地测状态机+UI,端到端需 EAS 真机+live 后端。

## Phase→Task
- **P1 基建(本地测)**:T1.1 call-source 加 FakeCall+loveRing 映射+isFakeCall;T1.2 ab-test 加 getAbBoolean+koda fake-call switch key(+清 lumi 残留);T1.3 report-user-events.ts 移植;T1.4 call-record.ts 移植 sendFakeCallRecord。
- **P2 恋爱铃状态机**:T2.1 love-ring.ts 移植(safe key/LumiEvent/koda AB/isFriend/门禁);T2.2 call-session beginCall 接结局回报;T2.3 use-call placeCall 余额不足回报。
- **P3 UI+接线**:T3.1 love-ring-banner 重画;T3.2 fake-call-host;T3.3 (tabs)/_layout 挂载;T3.4 logout resetLoveRing;T3.5 端到端真机验。
- **P4 孪生+抽包**:T4.1 full-screen-invite + pwa-video-recall(UI 重画、call-source 补 FullScreenInvite/PwaBackCall、互斥补回);T4.2 评估抽 headless。

**本地可测**:T1.*、T2.1 状态机、T2.3、T3.1 UI 预览、T3.4 幂等。**必须 EAS 真机+live 后端**:T2.2 完整结局、T3.5、P4。

## 风险
后端硬阻塞;AB key 归属;SecureStore key 合法字符(静默失败→频控永久失效);UserInfo 字段是否够填 banner;ab-test lumi 残留;孪生互斥(P4 延后需临时去引用+TODO);boundaries 门禁(不 import iris、不硬编码事件名/域名/key)。

## 进度
- 2026-09-07:方案定稿、范围=全做、开工 P1。分支 `feat/koda-love-ring`(从 feature/koda-android)。
- 2026-09-07:**Phase 1 完成**(未提交)。T1.1 `call-source.ts` 13 值枚举+全投影(+6 test);T1.2 `ab-test.ts` 加 `getAbBoolean`+`KODA_CONSUMED_AB_KEYS.fakeCallSwitch="g_fake_call_switch"`(+3 test);T1.3 `report-user-events.ts`(reportUserEvent→Alive C2SReportUserEvent,用 useSessionStore.userId 守卫)(+4 test);T1.4 `call-record.ts` `sendFakeCallRecord`(SendMessageFromBotRequest,非 cancelled 带 NoUnread)(+3 test)。biome/typecheck/boundaries/15 test 全过。**坑**:fork 执行 T1.2-1.4 时 0 改动(转派没做),改自己写。下一步 Phase 2。
- 2026-09-07:**Phase 2 完成**(未提交)。T2.1 `services/call/love-ring.ts` 全状态机移植(koda 化:useSessionStore/useKodaChatStore/LumiEvent(有 logBehaviourEvent 别名)/KODA_CONSUMED_AB_KEYS/AliveConnectionState 从 @heyhru/rn-alive;SecureStore 安全 key `koda_lovering_last_send_${uid}`(无冒号);内联 toInvite 投影 UserInfo→banner(去 candidate-mapper);门禁 couldShow 暂只 isLoveRingShowing()+P4 TODO 补 isFullScreen/isPwaRecall)(+12 test)。T2.2 `call-session.ts` beginCall 各结局 gated isFakeCall 回报(onCallEnd→ConnectedExit/onCallCancelled→NoAnswer/onUserReject→Rejected/onUserNoResponse→NotConnected(otherNoResponse)/onUserLineBusy→(otherLineBusy)/onDialError·!dispatched→(userCallFailed))。T2.3 `use-call.ts` 余额不足分支 isFakeCall→sendLoveRingAcceptInsufficientBalance。biome/typecheck/boundaries/330 test 全过(唯一失败 config.test.ts 串包 pre-existing)。koda alive manager API(on/send/subscribeState/getState)与 iris 一致;koda LumiEvent.FakeCall* 来自共享 business-analytics(koda 前缀,天然不串包)。下一步 Phase 3 UI。
- 2026-09-07:**Phase 3 完成**(未提交)—— 恋爱铃本体 P1-P3 全落地。T3.1 `components/call/love-ring-banner.tsx`(koda 暖色 StyleSheet 重画,不照搬 iris Uniwind/coral;`LoveRingHost` 逻辑同 iris:mount reportShowSuccess、10s 倒计时→autoDismiss+reportLoveBellClicked+sendCancelMessage、accept→placeCall(FakeCall)、decline→sendLoveRingCancel+reportLoveBellReject+sendDeclineMessage;埋点 LumiEvent.FakeCallPopup{Cancel,AcceptClick,CancelClick};avatar 用 @heyhru/rn-image+fallback;文案保留 iris 的"诚实推荐"不伪造实时 consent,kicker="SUGGESTED FOR YOU"不用"RUBY")。T3.2 `fake-call-host.tsx`(arm startLoveRing+挂 LoveRingHost;twins 留 P4)。T3.3 `(tabs)/_layout` 挂 FakeCallHost(与 MatchPushHost 并列)。T3.4 `app/_layout` guest 分支加 resetLoveRing()。biome/typecheck/boundaries/330 test 全过。**T3.5 端到端真机验待 EAS+后端推 S2C_LoveRingPush**。共享 business-analytics 有全 6 个 FakeCall 事件。下一步 Phase 4 孪生(full-screen-invite + pwa-video-recall + 互斥补回 + 抽包评估)。
