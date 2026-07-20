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


## ⭐ IndexedDB 存 TIM 消息:一条不可克隆 = 整个会话缓存永久写不进(2026-07-18 实测)

- **现象**:进会话写缓存 45 条,刷新回来 L2 只读到 37;**每次刷新都稳定 37**;缺的那段要等 revalidate 从服务端重拉。
- **根因**:`chatCacheDB` 的 `put` 抛 `DataCloneError` ——
  `Failed to execute 'put': function(e2){ this.instanceID=..., this.sizeType=.., this.size=.. } could not be cloned`。
  **IndexedDB 走结构化克隆、不接受函数**,而 TIM **图片/文件类**消息的 `payload` 上挂着 SDK 的类/方法。
- **致命点**:`put` 失败会让**整个事务**失败 —— 会话里只要有一条这种消息,整批就永远写不进去,盘上卡死在最后一次成功的版本;而 `flush()` 的 catch **完全静默**且自动回滚重试 → 永远失败、永远无声。
- **修法**:存盘前对每条 `raw` 做 JSON 往返只留可序列化数据(`formatMessage` 读的都是纯字段,不依赖原型/方法);**个别消息序列化失败就跳过它,不连累整批**。
- **配套**:`writeBatch` 要补 `tx.onabort` —— 配额超限走 abort 不走 error,只听 `onerror` 会让 Promise 永远悬着。`os.put` 对不可克隆值是**同步抛错**、不走 `tx.onerror`。
- **⭐⭐ 方法论**:我先后按「防抖没落盘」改了两版(加卸载兜底 flush、防抖改节流)都无效 —— 改的都是「**什么时候写**」,而真相是「**根本写不进去**」。**转折点是给静默的 catch 加日志**。一个吞异常的 catch 能让人朝错方向修无数遍;排查时先把「静默失败」暴露出来,再谈修法。

## 消息校准(reclaim)会误删「刚到、服务端还没索引」的消息

- `chatMessageSource.reclaim` 的判据是「在拉取前快照里 + 在窗口内 + 服务端页没有 = 已删」。但 TIM 实时推送往往**快过服务端落库/漫游索引**,这段窗口里拉历史根本拿不到它 —— 典型翻车:videoTips 卡由前端调 `/msgcenterApi/injectMessage` **现注入**,推送秒到、卡已渲染,紧接着一次 revalidate 就把它差集清掉(还连缓存一起写回),表现为「卡闪一下就消失,刷新后过一会儿又出现」。
- **修法**:`reclaim` 加豁免 —— 时间戳距今不足 60s 的一律保留,不交给服务端裁决。符合该函数既有的「宁可漏删不可误删」约束。
- **通用判据**:凡是「客户端能先于服务端看到」的消息(实时推送、现注入、乐观发送),都不能用「服务端拉不到 = 已删」来裁决。

## videoTips 卡的下发链路与「删不掉」

- **链路**:① 后端 `getConversationState` 返回含 `TASK_TYPE_VIDEO_TIPS` 的 task → ② **前端**调 `InjectMessage`(url `/msgcenterApi/injectMessage`)让后端注入 TIM 消息 → ③ 渲染成卡。
- **关键**:注入参数是 `fromUserId=peerUserId, toUserId=selfUserId`,即**以对方身份**注入 —— 女方端 `IMManager.deleteMessage` 删的是「别人发的消息」,`ok:true` 只代表 SDK 调用成功,**删不掉服务端漫游副本**。下次 `revalidate` 拉历史会把卡**合并回来**(实测:删卡后无任何新推送,却凭空多出同一个 message id)。
- **修法**:本地「已消费」黑名单(`videoTipsConsumed`,localStorage)+ 在 `filterMessages` 统一过滤(L1/L2/network 三条读路径都经它)。治本仍在服务端(收到 `phone_call_message.taskId` 后删掉那条注入消息 / 不再下发)。

## 会话锁(lockRef)会锁死切换,且绿灯时静默无提示

- 锁的条件只看**有没有 pendingTask**、不看颜色:`if (!pendingTask?.id) return; lockRef.current = { locked: true, status: convStatus }`。
- `handleSelectConversation` 被锁拦截时,**只有 Red/Yellow 弹 toast,Green 直接静默 `return`** —— 表现就是「点了完全没反应」。自动切换(`taskQueue` effect)同样 `if (lockRef.current.locked) return`。
- **解锁只在 ActiveChat 卸载时**(或 pendingTask 消失)→ 切不走就不卸载 → 不解锁 → 自我维持。
- **所以「PENDING 任务一直没被完成」会直接锁死会话切换。** 而前端完成 task 走的是发消息时 `buildCloudData` 的 `taskIds`(`event_type: "anomaly_task_complete"`),**和 `phone_call_message` 是两条独立的路** —— 「打完电话完成 PENDING 任务」必须**后端消费 `phone_call_message.taskId`** 才成立,前端单方面改不出来。
- **兜底选会话的 effect 也走 `handleSelectConversation`**,同样会被锁拦 —— 当前会话不在列表时本该自愈却被拦住,是既有隐患。

## 排查这块问题的姿势

- 全链路统一 tag 打点(`vtLog` → `[VideoTipsFlow]`),vConsole 搜一个词看完时间线:发卡 → TIM 到达 → 写缓存 → reclaim → 渲染 → 点击 → 通话 → phone_call_message → 删卡 → 清缓存。
- L2 读写**对账**日志是定位缓存问题的关键:写侧 `queued/truncated/droppedUnserializable`,读侧 `onDisk/afterTtl/afterFilter` + cutoff + 最老最新时间戳。`onDisk < 上次 queued` = 落盘丢失;`afterTtl < onDisk` = 被 TTL 砍;`afterFilter < afterTtl` = 被 filterMessages 砍。**一眼定责,不用猜。**
- 一键清缓存:`sitin4.clearChatCache()`(或 Debug 页「清除聊天缓存」)—— 一次清 L1 内存 + L2 IndexedDB(先 flush 再 clear)+ 5 个 localStorage key。注意已有的「清除 LocalStorage」**清不掉 IndexedDB 和内存**。

## 相关

- push / pre-push 卡无关包见 [sitin-next-push-prepush](./sitin-next-push-prepush.md)
- gh 身份切换见 [lark-cli-auth-and-perms](./lark-cli-auth-and-perms.md)
