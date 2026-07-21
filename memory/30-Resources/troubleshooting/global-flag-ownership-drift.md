---
title: 无归属的全局标记必然漂移 —— 加归属 + 时效 + 使用时复验
date: 2026-07-20
tags: [troubleshooting, 架构模式, 状态管理, sitin-next]
---

# 无归属的全局标记必然漂移

**2026-07-18 一天之内同一模式撞了 3 次，07-20 又用了第 4 次。** 见到这个形状就该警觉，不用等它出事。

## 模式识别（满足这三条 = 必然漂移）

1. 有一个**模块级 / 全局的单槽标记**（`let limited = false` 这种）
2. 「置位」和「消费」之间隔着**异步 + 多条路径**
3. 存在**「意图被吞掉但不发终止信号」的静默返回路径**（守卫早退、引擎未就绪、权限被拒、上游 catch 掉）

→ 标记不会被回收，**漂到下一个完全无关的对象上**。

## 四次实例（都在 sitin-next app-pwa 通话链路）

| # | 标记 | 漂移后果 |
|---|---|---|
| 1 | `useVideoTipsCall.activeRef` | 外呼权限拒/建单失败静默 return → 残留 true → 把下一通无关通话误当 videoTips（误挂断 + 误发奖） |
| 2 | `videoTipsCall.limited`（bool） | 发起意图那刻置 true，那通被双击守卫吞掉 → 漂到真正接通的另一通 → **把普通计费通话也 60s 强挂** |
| 3 | `pendingVideoTipsTaskId` | 4 条静默返回路径让登记无人回收 → 被下一通取走 → **把没打过的 task 上报成完成 + 误删卡** |
| 4 | `videoTipsCall.pendingJump` | 无归属 → `ActiveChat` 按会话 key 重挂载时，**别的会话**的挂载补取会消费掉它，对错误会话执行跳转 |

## 三层修法（逐层加固，缺一层就留口子）

### 第一层：补终止信号（不够，但必要）

失败路径不能静默 return，要发信号让状态机复位：

```ts
// 外呼权限拒 / 建单失败
eventBus.emit(EventNames.CALL_OUTGOING_ENDED, { reason: "aborted" });
```

**为什么不够**：只能覆盖你想到的失败路径。native 的 `onError` 压根不通知 web（端上只打日志），你补不了。

### 第二层：给标记加归属 + 时效

单槽 bool → 带身份的记录：

```ts
// ❌ let limited = false;
let limitedTargetId: number | null = null;   // 谁的
let limitedAt = 0;                            // 什么时候登记的
const LIMIT_INTENT_TTL_MS = 90_000;
```

消费时双重校验：

```ts
const stale = Date.now() - limitedAt > LIMIT_INTENT_TTL_MS;
if (stale || currentSession()?.remoteUserId !== String(limitedTargetId)) {
  reset();
  return;
}
```

### 第三层 ⭐：**校验要在「使用的那一刻」做，不能只在「登记的那一刻」做**

这是最容易漏的一层。第 2 次实例修完后，第 4 次仍出事，因为：

> 「对端匹配」校验只在 **arm 时**跑了一次，**开火时不复验**。
> native `onError` 不通知 web → `onStop` 不执行 → `limitedTargetId` 和 timer 双双残留 → 用户 60s 内重拨一通**普通**通话并接通 → 到点无条件挂断当前通话，**挂错人**。

修法是在 timer 回调里**再验一次**：

```ts
timer = setTimeout(() => {
  timer = null;
  // 登记与开火之间隔着不可控的时间和路径，必须复验
  if (currentSession()?.remoteUserId !== String(limitedTargetId)) {
    reset();
    return;
  }
  void hangupCurrentCall();
}, LIMIT_MS);
```

**通用原则：登记与使用之间隔着不可控的时间和路径，只在登记时校验等于没校验。** 这一层还能兜住未来新增的、你现在不知道的结束路径。

## 反例对照：同一文件里两套标准

第 4 次实例暴露出的问题 —— 同一个 `videoTipsCall.ts` 里：

- `takePendingVideoTipsTaskId` 有 `targetUserId` + TTL 双重校验 ✅
- `takeVideoTipsPendingJump` **什么校验都没有** ❌

**先修的那个学到了教训，新写的又踩回去。** 加固过的模式要形成本地约定，新增同类标记时照抄，而不是每次重新想。

## 检查清单

写下一个模块级可变标记时问自己：

- [ ] 它有归属吗（是谁的）？
- [ ] 它会过期吗（TTL）？
- [ ] 置位到消费之间，有没有「静默返回不发信号」的路径？
- [ ] **消费/开火那一刻**复验了吗，还是只在登记时验过？
- [ ] 同文件里已有的同类标记是怎么校验的，我一致吗？

## 相关

- [[pwa-call-order-lifecycle]] —— 挂载层决定生命周期（同一批排查的产物）
- [[sitin-next-pwa-chat-tim]] —— chat/TIM 侧的具体踩坑
