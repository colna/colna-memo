---
title: PWA「探真」验证任务抽屉(拍照 / 语音)
date: 2026-07-02
tags: [sitin4.0, app-pwa, chat, 探真, drawer, figma]
---

# PWA「探真」验证任务抽屉

AI 陪聊暂停时,让女方用**真实自拍 / 真实语音**回复对方,通过服务端审核(人脸/性别、语音内容)后发出,以此「探真」。两个底部抽屉,同构架构。

## 仓库 / 分支 / PR

- 仓库:`sitin-next` → `packages/app-pwa`,集成分支 `personal/zz/sitin4`。
- **拍照 PhotoTask**:分支 `personal/zz/pwa-verify-drawer`,**PR #517 已合并**。
- **语音 VoiceTask**:分支 `personal/zz/pwa-voice-verify-drawer`,**PR #518 已合并**。

## 已完成

- [x] `pages/ChatDetail/PhotoTaskDrawer.tsx` — 4 态(idle→ready→reviewing→failed),橙色主题,前置系统相机 `<input capture="user">`,详见 [[sitin-next-phototask-drawer]]。
- [x] `pages/ChatDetail/VoiceTaskDrawer.tsx` — 7 态(idle→recording→recorded→reviewing→success/failed;expired),紫色主题,`useVoiceRecorder` 动态声纹 + 回放播放头动画(整段解码),详见 [[sitin-next-voicetask-drawer]]。
- [x] `components/ModalContainer.tsx` 加 `dismissible?`(默认 true):false 时禁遮罩/下滑/Esc 关。两抽屉**关闭即重置**、**完成前不可关**。
- [x] dev 预览页 `/dev/photo-task`、`/dev/voice-task`。
- [x] 图标全 webp(遵循 [[sitin-next-pwa-figma-webp]]):PhotoTask 下 10 个、VoiceTask 复用 + 新增 1 个。

## 共用设计

- **注入式解耦**:抽屉不含后端,靠 `onVerify(...) => { passed, cdnUrl/voiceUrl?, reasons?, ... }`;ChatDetail 接真实 `uploadToOss + auditImage/auditVoice`,预览注入 mock。
- **防连点**:关键异步动作(send / 录音 start-stop)套 `useLockFn`。
- **倒计时**:`onCountdownEnd` 每 open 周期触发一次;photo 到点自动关,voice 到点进 expired 态。
- 移动端手势/媒体踩坑见 [[pwa-mobile-gesture-media]];push 卡 pre-push 见 [[sitin-next-push-prepush]]。

## 后端契约(2026-07-08 从飞书 sitin4.0 技术文档梳理)

探真=**探针(probe)任务**,是「异常任务」`AnomalyTaskItem` 的**附加属性,不独立存在**,详细后端全链路见 [[sitin4.0-anomaly-probe-backend]]。

- **触发链**:checker 打分(横/纵向)→ 红/黄/绿 → 高风险时 aichat 停代回 → sender → 后端建 `AnomalyTaskItem` 并附 `hasProbe=true`+`probeType` → IM 全局推送(不轮询)→ 前端刷 `/anomaly/list` 发现 hasProbe → 跳会话弹窗。
- **探针字段**(都在 `convData: AnomalyTaskItem`,proto 已生成):`hasProbe`、`probeType`(`ANOMALY_PROBE_TYPE_VOICE=1`/`IMAGE=2`)、`probePrompt`、`probeRewardCents`、`probeTimeoutSeconds`(默180)、`taskId`。
- 前端**只看 `hasProbe` 决定弹窗、不区分红黄绿**;弹窗不改会话状态,180s;**完成/清任务全靠后端收 TIM `AfterSendMsg` 回调**清整条任务(taskId+hasProbe)→ 推送 → 前端刷列表,**无独立清除接口**。
- **免审例外**:红色状态用户主动发图免审,但**探针图片仍需审核**(探真始终走 `useProbeTaskVerify`)。
- **审核接口**(chat-service-api gRPC,mock 先行 proto id):`GetUploadCredential`(23000,OSS presigned 5min)、`AuditText`(23001,Gemini)、`AuditVoice`(23002,Gemini 音频直审)、`AuditImage`(23003,数美 人脸+性别)。违规枚举与 `chatModerationApi.ts` 完全一致(`NO_HUMAN_FACE=5`/`NOT_FEMALE=6`)。

## PWA 接入方案(前端唯一缺口=触发/驱动接线)

现状:审核链、抽屉 UI、`/anomaly/list` 拉取(`chatConvsManager`→`convData`)、TIM 发送管线**全就位**;`hasProbe/probeType/...` 在业务代码**完全没用**;`Chat/index.tsx:189` 注释明示触发 unwired(`setShowPhotoTask` 从未被调);`anomalyStore` 工作台四态未建。

**分两期**:

- **P0(前端可独立交付)✅ 已实现**(commit `28df0aab`,分支 `personal/zz/pwa-probe-p0`,未 push):`Chat/index.tsx` 加触发 `useEffect`——`convData.hasProbe` 时按 `probeType` 弹对应抽屉,`handledProbeRef` 按 taskId 去重防重弹;抽屉补传 `rewardText/earnedText`(module helper `probeRewardLabel(probeRewardCents)`,cents/100,≥100 用 $)、`countdownSeconds`(`probeTimeoutSeconds ?? 180`)、`script`(probePrompt,**VoiceTask 的 prompt prop 叫 `script` 不是 promptText**)、`onSent/onCountdownEnd`(P0 仅 markHandled+关抽屉,发消息属 P1)。eslint 干净,tsc net-zero(stash 基线对比,既有 11 error 全是 `UserInfoWithCache` 类型基线)。mock 审核下走 `/dev/photo-task`、`/dev/voice-task` 全流程自测。
- **P1(依赖后端)**:proto 落地接真审核;`onSent` 复用 `IMManager.sendImageMessage/sendAudioMessage` 发出,**必带 `cloudCustomData{taskId, isProbeReply:true, isLastSegment:true, earnedCents}`**;与后端敲定**发消息通道**——文档主张经消息中心 `POST /messages/downlink`(女方身份代发+`ForbidAfterSendMsgCallback` 防回环),当前 PWA 是直发 TIM,**需确认**。
- **P2(随真人接管)**:建 `stores/anomalyStore.ts`(`highestPriority/targetUserId/isProcessing`),跨会话按优先级 `Red>Yellow>Green`(同级 hasProbe 优先)自动 `navigate('/chat',{state:{conversationId,from:'workspace_task'}})` 跳转+弹窗。归真人接管 owner 耿学岩范畴。

**代码改动清单**:`pages/Chat/index.tsx`(触发+抽屉入参)、新 `useProbeSend.ts`(onSent 发消息+cloudCustomData)、`services/IMManager.ts`(`sendImageMessage/sendAudioMessage` 补 `cloudCustomData` 透传,**待确认现有是否透传**;`createTextMessage` 已有 injectSource)、`centsToText` 复用。

**依赖/阻塞**:①`chat_api.proto` 未落地(owner 钱文锦,`pnpm proto:gen` 前 mock);②发消息通道 downlink vs 直发 TIM 待定;③cloudCustomData 契约需与后端 `AfterSendMsg` 解析对齐;④P2 依赖真人接管整体。

## Follow-up(前端剩余)

- [ ] 回放波形已整段解码;真机 visual 校验(iOS 麦克风/相机权限)。
- [ ] 抽屉从 `pages/ChatDetail/` 已随路由重命名到 `pages/Chat/`(本笔记旧路径已过时)。

## 相关

- 前置项目:[[pwa-chat-input-bar]](底栏 ChatInputBar / ChatVoiceRecorder,PR #499)。
- 后端全链路契约:[[sitin4.0-anomaly-probe-backend]]。
- 工作日志:[2026-07-02 Daily](../50-Daily/2026-07-02.md)、[2026-07-08 Daily](../50-Daily/2026-07-08.md)。
