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
- commit 序列:`3591d6bc`(组件+预览)→`075b8dbe`(接入 ChatDetail、删 ChatFooter)→`d69a49a9`(语音状态机)→`53ce1064`(修 async event currentTarget)→`3d583903`(透明底栏+白圆按钮)→`48629b70`(channel 居中去背景)→`c55dfe76`(blob 日志)→`aa683098`(波形从左往右)→`c6ff580b`(按钮原地放大 scale)→`cc1b1f3f`(语音条阴影)→`c5c4b3ae`(激活按钮阴影)→`f997f963`(修 setPointerCapture 竞态)→`06883042`(修长按弹图片菜单)→`cf9acfdc`(录制零延迟)→`196049c6`(修声纹不动 AudioContext resume)

## 已完成

- [x] 从 Figma REST 导出 5 个图标 → sharp 无损转 webp:`src/assets/images/chat/icon_chat_{keyboard,mic,mic_active,camera,gift_line}.webp`
- [x] `src/components/ChatInputBar.tsx` 独立组件:text/voice 两态(默认 text),回调式 API `onSendText` / `onGiftClick` / `onPickImage`,可选 `freeVoiceCount`(默认 3)/`defaultMode`
  - 文字态:蓝底麦克风切换 + 输入框 + 相机 + 礼物(回车发送)
  - 语音态:键盘切换 + "Hold to talk" 淡蓝胶囊 + `×N` free 标签 + 相机 + 礼物
  - 相机:内置 `<input type=file accept=image/*>`,移动端调起系统拍照/相册
- [x] dev 预览页 `src/pages/ChatInputBarPreview/` + 路由 `/dev/chat-input-bar`(支持 `?mode=voice`),两态截图已 1:1 还原 Figma
- [x] 接入真实聊天页 `ChatDetail/index.tsx`:替换 `ChatFooter` → `ChatInputBar`,礼物复用现有 `GiftList` 弹窗(送礼/余额/对象真实生效),`pwa_chat_send_message` 埋点搬到页面层;删除死代码 `ChatFooter.tsx`
- [x] **语音录制状态机(真实录音)**:`hooks/useVoiceRecorder.ts`(MediaRecorder 采集 + AudioContext AnalyserNode 真实振幅波形 + 计时,`blob.size>0` 才 resolve,suspended 时 `resume()`)+ `components/ChatVoiceRecorder.tsx`(pointer 手势状态机:按住录制→上滑 56px 锁定 / 112px 取消→松手 send/discard,<1s too-short toast;持久 root 承接 pointer capture;按下零延迟进态、getUserMedia 后台化)+ 12 个 webp 图标(含 voice_ 系列);`onSendVoice(blob,durationMs)` 回调占位(ChatDetail 打 blob 详情+可播放 url 日志)
- [x] **样式还原 Figma**(多轮迭代):底栏透明只按钮白圆底、语音条/send/激活按钮 drop-shadow、波形从左往右 + 密集蓝竖条、slide-channel 去胶囊背景 + 屏幕水平居中、trash/lock 激活 `transform scale` 原地放大不挤兄弟
- [x] **真机 bug 修复**:async event `currentTarget` 回收、`setPointerCapture` NotFoundError 竞态、长按 `<img>` 弹浏览器图片菜单(`[&_img]:pointer-events-none` + `onContextMenu`)、录制零延迟、声纹不动(AudioContext resume)

## Follow-up(未做)

- [ ] 相机选图后的 OSS 上传 + 发送图片消息(现为 `onPickImage` 占位 `console.log`+TODO;可复用 `utils/ossUpload.uploadToOss`)
- [ ] 语音消息的 OSS 上传 + 发送(现为 `onSendVoice` 占位;录制/手势/波形/blob 获取已完成并真机验证)
- [ ] iOS 真机验证(本会话真机测试主要在 Android Chrome;iOS 可能还需 `-webkit-touch-callout:none` 等)

## 关键设计决策

- **组件纯净**:`ChatInputBar` 只暴露回调,不绑业务;`GiftList`、埋点、发送逻辑都留在页面层。接入 = 替换挂载点 + 接线页面已有逻辑,组件零改动。
- **图标 webp 而非 svg**(需求要求):Figma `/v1/images` 导 PNG(scale=4,图标容器白底 `visible:false` → 透明)→ scratchpad 临时 `sharp({lossless:true})` 转 webp。
- **验证走 build+preview**:app-pwa `vite dev` 在本机 hoisted monorepo 下 `_jsxDEV is not a function` 白屏(环境级,基线 `/debug` 同崩),视觉验证用 `vite build --mode development` + `vite preview` + puppeteer(系统 Chrome)。

## 相关

- 工作日志详见 [2026-07-01 Daily](../50-Daily/2026-07-01.md)(03:45 实现 / 04:10 接入)
- 上层依赖:`GiftList`(`ChatDetail/GiftList.tsx`)、OSS 直传 `utils/ossUpload`
