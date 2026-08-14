---
title: sitin4.1 CE 交换 / 社媒授权任务 踩坑
tags: [sitin4.1, app-pwa, ce, social-auth]
---

## SocialAuthTaskDrawer(聊天页「{名}'s {平台} request」授权卡)不弹的真因(2026-08-12)
- **现象**:后端明明下发了社媒授权任务,聊天页那张授权抽屉(SocialAuthTaskDrawer)就是不弹。
- **根因**:前后端字段口径不一致。后端 `GetConversationState` 的 `tasks[]` 项是 `{id, reasons:["{\"kind\":\"social_account_authorization\",\"platform\":\"ig\"}"], createdAt, updatedAt}`——信息在 `reasons[]` JSON 字符串里,**无 taskType / socialPlatform**;而前端 `utils/socialAuthTask.ts` 判 `taskType===4` + 读 `task.socialPlatform` → 识别不出 → drawer open=false。
- **修法**:前端解析 `reasons[]`(找 kind===social_account_authorization 读 platform)。文件注释原写「proto 尚未下发 taskType=4/socialPlatform,后续换生成枚举」——但后端走了 reasons 路线,需按实际契约适配。
- **排查手法**:vConsole 导出的日志里对象被打成 `Object` 折叠、文本里读不到;要在 vConsole 展开 GetConversationState 响应复制 JSON,才看得到 tasks 真容。

## 聊天页刻意不弹交换大抽屉(2026-08-12)
- `useInsTaskInit.canShowInsModal()` 对 `pathname.includes("/chat/")` 直接 return false → 聊天页不弹「Social Requests」大抽屉。
- 聊天页 CE:有平台在线→后台自动派发接受(dispatchInsExchangeOrders,发 INS_FOLLOW_REWARD);呈现=内嵌卡 InsExchangeBubble + 底部 SocialAuthTaskDrawer,不是大抽屉。

## 社媒授权「加钱弹窗」唯一触发点(2026-08-12)
- 加钱弹窗只由 `InsModalContent.handleComplete`(showInsModal.tsx)在弹窗内「未授权→已授权」时调 `finishTask(taskId)` 触发;`finishPWAInsTask` 回调不调 finishTask。故非弹窗完成路径(webview 重登/已授权 Reconnect)不弹加钱窗。金额=`parseFloat(task.reward)`,后端 reward=0/空则即使触发也不弹。
