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

## Follow-up(未做)

- [ ] **接入 ChatDetail**:探真消息气泡触发抽屉、倒计时到期时间 / 奖励(¢1.20 / ¢0.80)/ taskId 的**后端数据源**、审核通过后发消息、填掉 `handlePickImage` / `handleSendVoice` 的 TODO。
- [ ] `auditImage` / `auditVoice` 依赖的 `archat_api/chat_api` proto **后端尚未落地**,真机审核要等 `chat_api.proto` + `pnpm proto:gen`。
- [ ] 回放波形已整段解码;真机 visual 校验(iOS 麦克风/相机权限)。

## 相关

- 前置项目:[[pwa-chat-input-bar]](底栏 ChatInputBar / ChatVoiceRecorder,PR #499)。
- 工作日志:[2026-07-02 Daily](../50-Daily/2026-07-02.md)。
