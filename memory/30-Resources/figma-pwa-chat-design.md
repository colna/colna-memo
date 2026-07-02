---
title: Figma —— PWA 聊天/输入栏设计源(pwa整理)
date: 2026-07-02
tags: [figma, design, pwa, sitin-next, chat, reference]
---

# Figma:PWA 聊天设计源「pwa整理」

对应代码:`sitin-next/packages/app-pwa`(分支 `personal/zz/pwa-chat-bottombar`),组件 `ChatInputBar.tsx` / `ChatVoiceRecorder.tsx` / `hooks/useVoiceRecorder.ts`。

## 文件

- **名称**:pwa整理
- **fileKey**:`eQTrA7uTDAn6Jhe9dSXaeZ`
- **URL 模板**:`https://www.figma.com/design/eQTrA7uTDAn6Jhe9dSXaeZ/pwa%E6%95%B4%E7%90%86?node-id=<id>&m=dev`
- **取数**:figma MCP(`get_figma_data`)需先 `/mcp` 重连(token 见 `~/.claude.json`);MCP 不可用时可直接 curl:
  `curl -H "X-Figma-Token: <token>" "https://api.figma.com/v1/files/eQTrA7uTDAn6Jhe9dSXaeZ/nodes?ids=<id>"`(node id 里的 `-` 换成 `:`)。

## 语音录制 4 态 node 映射(frame 前缀 4139)

| 态 | node-id |
|---|---|
| 录制中 Recording (S1) | `4139-13572` |
| 上滑取消 | `4139-13640` |
| 锁定免提 | `4139-13690` |
| 时间太短 | `4139-13758` |
| slide-channel(侧滑胶囊,含 #EFEFF4 底) | `4139-13679` |

## 已量规格(录制中 4139-13572,2026-07-02 核对,代码已对齐)

- 输入行:HORIZONTAL,itemSpacing **10**,padding-x **16**,居中。
- voice-field pill:**44** 高,圆角 **999**,padding-x **14**,itemSpacing **8**,底色 **#EBF5FF**,阴影 `0 2 8 rgba(16,152,250,0.04)`。
- pill 内:mic **18**、计时文字 **15px/600/#1098FA**、waveform、trailing chevron **16**。
- waveform:容器高 **24**,条宽 **2**,itemSpacing **2**,圆角 1,色 #1098FA。
- 侧滑 slide-channel:52×112,圆角 26,底 `#F0F0F5@55%`;cancel/lock target 各 **40×40** 圆角 20、白底 + 描边 #E3E6EC、图标 **18**、纵向间距 **12**。
- 相机/礼物/toggle 按钮:**40×40** 圆、白底、描边 #E3E6EC。

## 代码里有意偏离(用户拍板,勿"改回对齐")

- **波形条距**:代码 `gap-px`(1px),Figma 是 2px —— 2026-07-01 用户要更密。
- **侧滑胶囊底**:代码去掉了 `#F0F0F5@55%` 底(只按钮各自有底)—— 用户看实物要求去掉。

相关踩坑见 [[pwa-mobile-gesture-media]]。
