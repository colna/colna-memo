---
title: sitin-next app-pwa 聊天页 / TIM 排错经验
date: 2026-07-06
tags: [troubleshooting, sitin-next, app-pwa, tim, chat]
---

# sitin-next app-pwa 聊天页 / 腾讯 TIM 排错

聊天页发送链路(text/voice/image/gift)+ 消息缓存的复用经验。真源代码在 `sitin-next/packages/app-pwa/`。

## TIM 发图片/语音/文件报「未检测到上传插件」

- **现象**:`IMManager.sendImageMessage failed ni: 未检测到上传插件`(sendAudio/sendFile 同);纯文本能发。
- **根因**:腾讯 IM Web SDK 发**非文本**消息(图片/语音/文件走腾讯 COS 上传)必须 `chat.registerPlugin({'tim-upload-plugin': TIMUploadPlugin})`。项目 `IMManager.create()` 后没注册,`package.json` 也没装 `tim-upload-plugin`。纯文本不经上传故不受影响。
- **修法**:`pnpm add tim-upload-plugin -F @heyhru/app-pwa`,`IMManager.create()` 后 `chat.registerPlugin(...)`。
- **坑**:`IMManager` 三个 send 方法当时是 `catch { return null }` **静默吞错**,上层 `sendImage` 拿到 null 不 throw、UI 照打 `image.onSendSuccess`,控制台无报错 → 排查第一步是给空 catch 加 `console.log(TAG, "sendXxx failed", err)` 让错误可见。

## 发礼物成功扣费了但聊天里没有礼物气泡

- **现象**:`PWASendGift` 返回 `code:1`、余额扣费推送 `SEND_GIFT`、`gift.onSendSuccess` 都正常 = **发送成功已扣费**,但聊天里不出现礼物气泡。
- **根因**:礼物气泡靠一条 `CustomDescription.Gift` 的 **TIM 自定义消息**渲染(`bubbles/index.tsx` MessageType.Gift→GiftBubble)。`sendGift`(HTTP)只负责扣费,**不发 TIM 消息**;后端 PWASendGift 也不回推礼物消息给自己端 → 没有消息就没有气泡。
- **修法**:`handleSendGift` 在 `sendGift` 成功后补发 `useSendMessage().sendGiftMessage(toUserId, giftImageUrl, priceDollar)`(内部 `IMManager.sendCustomMessage` + `broadcastMessage` 让发送方自己也收到 → 渲染气泡 + 触发动画)。此方法早已存在,之前只在 ins 交换用,普通聊天发礼物漏调。
- **前提**:礼物 TIM 消息由**前端**发、后端只扣费。若后端其实也推,对方会看到两条,需后端确认。

## TIM login/connect 不能缓存,但可以「不让 UI 等它」

- 刷新 = 新 JS 实例、WebSocket 断、内存清零 → TIM SDK 必然重新 login/connect(几秒),**这步不能缓存**(SDK 架构决定)。
- 但可优化感知:①`userSig` persist(省换签名);②TIM 自带 IndexedDB 本地存储(ready 后数据秒回);③关键是**入口缓存** —— 让 UI 用本地缓存先渲染、login 后台并行、ready 后 reconcile,而不是等 SDK ready 才 mount 聊天页。
- 具体:`activeConversationId` 用 `useState(()=>localStorage.getItem(...)??"")` 同步恢复 → `ActiveChat key={convId}` 立即 mount → 读 L2 缓存消息在 TIM ready 前先画;会话列表 `chatConvsStore` 用 zustand `persist`(partialize 存前 50)同步 rehydrate → 顶部头像栏刷新首屏就有。

## 两级消息缓存的关键坑

- **TimMessage 是 class**(带 `id`/`timestamp`/方法),**不能直接进 IndexedDB**(结构化克隆丢方法)→ 存 `msg.raw`(原始 Message),读出用 `formatMessage(raw)` 重建。
- 50 条硬截断有游标缺口 → L2 存最近 50 + `hasMore=true`、**不缓存深翻游标**,靠 reconcile 拉最新页重新派生 cursor 规避 gap。
- L1(zustand Map,LRU 20,命令式 getState 读写不驱动渲染)消灭切换闪烁;L2(手写 IDB,dirty 队列 + 1.5s 防抖批量 flush,schemaVersion 失效即 miss)扛冷启动。
- 缓存 key 必须加 **userId 前缀**(`${userId}:${convId}`),登出 `clear()` 防串号。
- 缓存生效诊断日志:TAG=`ChatCache` 打 `L1 hit (memory)` / `L2 hit (IndexedDB)` / `miss → network` / `reconciled (bg refresh)`;DevTools→Application→IndexedDB→`chat-cache` 看数据。

## 输入框发送后不清空(移动端软键盘竞态)

- **现象**:文字发送后输入框没清空,还容易连发。日志 `onStart→onSend→onSendSuccess`(无 onSendError)证明 `setInputText("")` 跑了、没抛错、值被**外部写回**。
- **根因**:软键盘竞态。keydown 里清空 state,但键盘紧接着 fire 一条 trailing input/composition 事件把文字同步回受控 input(await 期间 DOM 还没 flush)。
- **修法**:①`flushSync(()=>setInputText(""))` 在 keydown 栈里同步清空 DOM,抢在 trailing 事件前;②Enter 加 `e.nativeEvent.isComposing` 守卫(IME 合成中不发送)。若键盘主动写回(非 re-sync),还需加"忽略已发送文字的回显 onChange"兜底。

## `ActiveChat key={convId}` 下的 reconcile 竞态其实不会发生

- review 常报"异步 reconcile 闭包捕获 convId,快速切 A→B 时旧回调污染 B"。**在 `key={activeConversationId}` 重建模型下不会实际发生**:每次切换是新 `useChatMessages` 实例、旧实例卸载,旧 reconcile 的 setMessages 作用于已卸载实例 = React no-op,ref 每实例独立。
- 单纯加 `conversationIdRef` 对比在 key 重建下**不生效**(旧实例 ref 恒 = A),它只防"同实例复用"。
- 稳妥双保险:`aliveRef`(unmount cleanup 置 false)覆盖当前 key 重建;`convId !== conversationIdRef.current` 对比覆盖未来去 key 复用。`isStale(convId)=!aliveRef.current||convId!==ref` 在每处 await 后 bail。

## authBlock 反逻辑 crash(base feature/sitin4.0 既有 bug)

- **现象**:正常用户进聊天页 `Cannot read properties of undefined (reading 'title')` @ `AIPersonaPlaceholder.tsx`。
- **根因**:`Chat/index.tsx` `if (!authBlock)` 写反了 —— 正常用户 `authBlock=null` → `!null`=true 进 placeholder 分支 → `variantMap[null]`=undefined → 崩。应为 `if (authBlock)`。
- **注意**:此 bug 在 base `feature/sitin4.0` 主分支也有,需告知团队。

## 聊天审核(useChatModeration)

- 真实审核 hook `useChatModeration().runModeration({ type, targetId, text|file })`:text→`auditText`;voice/image→`uploadToOss` 后 `auditVoice`/`auditImage`。返回 `AuditResult { passed, violationCategory, violationMessage }`(**无 `reason` 字段**,拒绝理由用 `violationMessage`)。错误 throw,由调用方决定 fail-open/closed。
- `targetId` 是对方 userId(必填);footbar 里 `targetId: convToUid(conversationId)`。
- 产品约定:**图片在 footbar 不审核**(sendImage needCheck 默认 false),但探真抽屉(PhotoTaskDrawer)内要审核。
- `chatModerationApi` 的 `chat_api` proto **已生成**(`src/gen/`、`dist/gen/`),旧注释"尚未生成"过时。
- mock 桩 `useMsgCheck`(恒 `passed:true`)已删。

## 相关

- push / pre-push 卡无关包见 [sitin-next-push-prepush](./sitin-next-push-prepush.md)
- gh 身份切换见 [lark-cli-auth-and-perms](./lark-cli-auth-and-perms.md)
