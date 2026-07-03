---
title: sitin-next app-pwa — VoiceTaskDrawer(探真语音抽屉)使用方式
date: 2026-07-02
tags: [sitin-next, app-pwa, component, voicetask, 探真]
---

# VoiceTaskDrawer 使用方式

「探真」语音验证底部抽屉(紫色主题)。文件 `packages/app-pwa/src/pages/ChatDetail/VoiceTaskDrawer.tsx`(PR #518,分支 `personal/zz/pwa-voice-verify-drawer`)。与 [[sitin-next-phototask-drawer]] 同构。

## Props

```ts
interface VoiceVerifyResult {
  passed: boolean;
  voiceUrl?: string;    // 通过时给出
  reasons?: string[];   // 失败红 chip(如 ["Flagged words","Not allowed"])
  transcript?: string;  // 失败 TRANSCRIPT 卡文案(可含 ███ 屏蔽)
}

interface VoiceTaskDrawerProps {
  open: boolean;
  onClose: () => void;
  peerName: string;                 // "Send Jake a voice note"
  script?: string;                  // "SAY THIS" 提示脚本
  rewardText?: string;              // 默认 "Earn +¢0.80"
  earnedText?: string;              // 成功页 "Earned +¢0.80"
  countdownSeconds?: number;        // 默认 45
  onVerify: (blob: Blob, durationMs: number) => Promise<VoiceVerifyResult>;
  onSent?: (voiceUrl: string) => void;
  onCountdownEnd?: () => void;      // 到 0 触发一次
}
```

## 状态机(内部自管)

`idle(待录音) → recording(录音中·动态声纹) → recorded(回放) → reviewing(审核中) → success / failed ; expired`

- **idle**:SAY THIS 脚本卡 + 倒计时 + Record 按钮。**点击 Record 开始录制,不长按**。
- **recording**:`useVoiceRecorder.levels` 渲染动态声纹条 + REC 计时 + Stop &amp; review。
- **recorded**:回放卡(播放键 + 波形 + 时长)+ 绿 chip + Send。**回放有播放头动画**:停录后 Web Audio 解码整段成 44 根峰值,播放时按 `currentTime/durationMs` 从左到右扫过(已播 `#6C4CF0`、未播 `#CDC4F4`)。
- **recorded**:回放卡。**点整个音频条 = 播放/暂停回放**;左侧圆 reload 按钮 = 重新录制(回 idle)。实现:卡片 `relative` + `absolute inset-0` 透明按钮当整条播放热区(定位元素绘制在波形之上,点波形区也命中),reload 按钮 `relative z-10` 顶上层;用兄弟按钮非嵌套。图标 `icon_voicetask_rerecord.webp`。对齐 Figma `4139-14261`。
- **reviewing**:调 `onVerify`;灰化 + 禁用 Send(播放/重录也 disabled)。
- **通过** → success 居中态(Done);**失败** → failed(TRANSCRIPT 卡 + Re-record)。
- **expired**:倒计时到 0 进此态(Got it)+ 触发 `onCountdownEnd`。
- **success/expired 两态与 PhotoTaskDrawer 共用** `pages/ChatDetail/taskDrawerStates.tsx`(`TaskSuccessBody`/`TaskExpiredBody`,只传文案);详见 [[sitin-next-phototask-drawer]]。

## 弹窗行为(与 PhotoTask 一致)

- **关闭即重置**:`open` 变 false 时清状态 + 释放录音/objectURL;重开干净。
- **完成前不可关**:`ModalContainer` 的 `dismissible` 设为 `terminal`(仅 success/expired 可点遮罩/下滑/Esc 关);其余态只能重录或等到期。

## 接真实后端(ChatDetail 用)

`onVerify` 串 OSS 直传(`FileType.VOICE`)+ `auditVoice`(注意 proto 后端未落地,等 `chat_api.proto` + `pnpm proto:gen`):

```ts
const cred = await uploadToOss({ file: blob, fileType: FileType.VOICE, fileExt: "webm", contentType: blob.type });
const audit = await auditVoice({ userId, voiceUrl: cred.cdnUrl, targetId: peerUserId });
// audit.passed / audit.violationCategory / audit.violationMessage
```

## Dev 预览

`/dev/voice-task`(`pages/VoiceTaskPreview/index.tsx`),mock `onVerify` 可切通过/失败,需麦克风权限。

## 已知坑

- MediaRecorder webm **无 duration 头** → `audio.duration = Infinity`;回放进度必须用录制测得的 `durationMs` 算,不能用 `audio.duration`。
- `useVoiceRecorder.levels` 是滚动窗(最近 ~2.9s),只适合「录音中」实时展示;回放静态波形要用 `decodeAudioData` 解码整段(已实现)。
