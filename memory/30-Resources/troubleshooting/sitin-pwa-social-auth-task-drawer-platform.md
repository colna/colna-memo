---
title: sitin-pwa chat 强制授权任务抽屉不弹(平台真源在 reasons)
date: 2026-08-17
tags: [troubleshooting, sitin-next, app-pwa, ce, social-auth, FE-2.2]
---

# SocialAuthTaskDrawer(chat CE 强制授权任务)不弹

## 现象
后端 convlist(GetConversationState)已下发社媒授权任务,但 chat 页那张
`SocialAuthTaskDrawer`(「{名}'s {平台} request」)不弹。

后端 task 结构:
```json
{ "taskType": 4, "socialPlatform": "",
  "reasons": ["{\"kind\":\"social_account_authorization\",\"platform\":\"snapchat\"}"],
  "timeoutSeconds": 0, "reward": [] }
```

## 根因(前后端字段口径不一致)
前端 `utils/socialAuthTask.ts` 的 `readTaskSocialPlatform` 只读 `task.socialPlatform`,
后端该字段**恒为空串**;真正的平台在 **`task.reasons[0]` 的 JSON 串**里。
→ `parseSocialAuthPlatform("")` = null → `isSocialAuthTask` false →
`getSocialAuthTask` 返 undefined → 抽屉 `open = !!task && !!platform && …` 恒 false → 永不弹。

(`taskType===4` = SOCIAL_ACCOUNT_AUTHORIZATION 本身匹配;`timeoutSeconds:0`→无倒计时不算过期;
`reward:[]`→显示 ¢0.00,都不影响弹出。唯一卡点是平台解析。)

## 修法(commit 116065c14)
`readTaskSocialPlatform` 改为优先从 reasons 解析:
```js
for (const raw of task.reasons ?? []) {
  const p = JSON.parse(raw);            // try/catch
  if (p?.kind === "social_account_authorization" && p.platform) return p.platform;
}
// 回退 task.socialPlatform(目前恒空)
```

## 经验
- FE-2.2 这类「后端约定但 proto 未正式下发」的字段,取值口径要以**后端实际返回**为准,
  别照 proto/约定名想当然。平台真源 = `reasons` JSON,不是 `socialPlatform`。
- 全新克隆跑 app-pwa tsc 前,先 `pnpm build` 内部 workspace 包(common-util-format / web-util-media 等),
  否则报 TS2307 找不到模块(与业务改动无关)。
- 多分支**共享一个工作区**易串台;隔离改动可另开干净克隆(如 sitin-next3)或按文件 stash。
