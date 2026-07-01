---
title: PWA 聊天底栏 ChatInputBar
date: 2026-07-01
tags: [sitin4.0, app-pwa, chat, ui, figma]
---

# PWA 聊天底栏 ChatInputBar

## 目标

按 Figma(pwa整理 文件,node `4139-13510` 语音态 / `4139-13551` 文字态)实现一个聊天底栏组件:语音/文字两态切换、相机点击在手机端调起系统拍照/相册、礼物复用 `/chat-detail` 的 `GiftList` 弹窗。图标从 Figma 抽取转 **webp**(不用 svg)。

## 仓库 / 分支 / PR

- 仓库:`sitin-next` → `packages/app-pwa`
- 分支:`personal/zz/pwa-chat-bottombar`(从集成分支 `personal/zz/sitin4` 切出)
- **PR #499**:https://github.com/presence-io/sitin-next/pull/499(base `personal/zz/sitin4`,colna 账号提)
- 关键 commit:`3591d6bc`(组件+预览页)、`075b8dbe`(接入 ChatDetail、删 ChatFooter)

## 已完成

- [x] 从 Figma REST 导出 5 个图标 → sharp 无损转 webp:`src/assets/images/chat/icon_chat_{keyboard,mic,mic_active,camera,gift_line}.webp`
- [x] `src/components/ChatInputBar.tsx` 独立组件:text/voice 两态(默认 text),回调式 API `onSendText` / `onGiftClick` / `onPickImage`,可选 `freeVoiceCount`(默认 3)/`defaultMode`
  - 文字态:蓝底麦克风切换 + 输入框 + 相机 + 礼物(回车发送)
  - 语音态:键盘切换 + "Hold to talk" 淡蓝胶囊 + `×N` free 标签 + 相机 + 礼物(**录制/发送仅静态 UI**)
  - 相机:内置 `<input type=file accept=image/*>`,移动端调起系统拍照/相册
- [x] dev 预览页 `src/pages/ChatInputBarPreview/` + 路由 `/dev/chat-input-bar`(支持 `?mode=voice`),两态截图已 1:1 还原 Figma
- [x] 接入真实聊天页 `ChatDetail/index.tsx`:替换 `ChatFooter` → `ChatInputBar`,礼物复用现有 `GiftList` 弹窗(送礼/余额/对象真实生效),`pwa_chat_send_message` 埋点搬到页面层;删除死代码 `ChatFooter.tsx`
- [x] 语音录制状态机(commit `d69a49a9`):`hooks/useVoiceRecorder.ts`(MediaRecorder+AudioContext 波形+计时)+ `components/ChatVoiceRecorder.tsx`(pointer 手势:按住录制→上滑锁定/取消→松手 send/discard/too-short)+ 7 个 webp 图标;`onSendVoice(blob,durationMs)` 回调占位

## Follow-up(未做)

- [ ] 相机选图后的 OSS 上传 + 发送图片消息(现为 `onPickImage` 占位 `console.log`+TODO;可复用 `utils/ossUpload.uploadToOss`)
- [ ] 语音消息的 OSS 上传 + 发送(现为 `onSendVoice` 占位;录制/手势/波形已完成)
- [ ] 录制态交互 UI 的真机/登录态验证(headless 预览页受 App 未登录重定向阻挡,未能自动截图)

## 关键设计决策

- **组件纯净**:`ChatInputBar` 只暴露回调,不绑业务;`GiftList`、埋点、发送逻辑都留在页面层。接入 = 替换挂载点 + 接线页面已有逻辑,组件零改动。
- **图标 webp 而非 svg**(需求要求):Figma `/v1/images` 导 PNG(scale=4,图标容器白底 `visible:false` → 透明)→ scratchpad 临时 `sharp({lossless:true})` 转 webp。
- **验证走 build+preview**:app-pwa `vite dev` 在本机 hoisted monorepo 下 `_jsxDEV is not a function` 白屏(环境级,基线 `/debug` 同崩),视觉验证用 `vite build --mode development` + `vite preview` + puppeteer(系统 Chrome)。

## 相关

- 工作日志详见 [2026-07-01 Daily](../50-Daily/2026-07-01.md)(03:45 实现 / 04:10 接入)
- 上层依赖:`GiftList`(`ChatDetail/GiftList.tsx`)、OSS 直传 `utils/ossUpload`
