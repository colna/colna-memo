---
title: useEffect 里的计时器 + deps 含回调 = 永远不触发
date: 2026-07-09
tags: [troubleshooting, react, hooks, sitin-next, app-pwa]
---

# 计时器 effect 依赖了回调，父组件周期性重渲染 → 计时器被无限重置

## 症状

自动消失的弹窗永远不消失。不报错、不告警、React DevTools 里看不出异常。

## 机制

```tsx
// 调用方：内联箭头，每次渲染都是新引用
<RewardPopup onClose={() => { setRewardPopup({open:false}); ... }} />

// 组件内：依赖它
useEffect(() => {
  if (!open) return;
  const t = setTimeout(onClose, 2000);
  return () => clearTimeout(t);
}, [open, onClose]);        // ← onClose 每次渲染都变
```

只要父组件**周期性重渲染**（倒计时 `setInterval`、轮询、ws 推送），
每次渲染 → `onClose` 新引用 → effect 重挂 → `clearTimeout` 掉旧计时器 → 从零重新计时。

**重渲染周期 < 计时器时长 ⇒ 永远到不了。**

实例：`useTaskCountdown` 每 1000ms tick → `setReplyWindow` → Chat 每秒重渲染；
弹窗的 1.6s 淡出与 2.0s 关闭双双失效。

## 修法

回调进 ref，deps 只留真正的「开关」state：

```tsx
const onCloseRef = useRef(onClose);
onCloseRef.current = onClose;      // 每次渲染同步，不进 deps

useEffect(() => {
  if (!open) return;
  const t = setTimeout(() => onCloseRef.current(), 2000);
  return () => clearTimeout(t);
}, [open]);
```

## 规则

1. **`useEffect` 里起计时器 / 订阅 / 动画，deps 绝不能含回调 prop。** 存 ref。
2. **组件要能扛住调用方传内联函数。** 不能假设 caller 写了 `useCallback` —— 存 ref 是组件自己的责任。
3. **删掉兜底交互前，先确认主路径是对的。**
   这个依赖一直是错的，但旧弹窗走 `ModalContainer`，有遮罩可点掉，症状被兜住了。
   照原型改成 `pointer-events:none` 后，计时器成了唯一出路 → 从「计时不准」升级成「永远关不掉」。
   **「移除逃生出口」会把潜伏 bug 变成致命 bug。**

## 反向的另一半：effect 读了什么状态，就必须**订阅**什么状态（2026-07-20）

上面讲的是「deps 不能多」（含回调 → 无限重挂）。这条是「**deps 不能少**」，症状完全相反、更隐蔽。

**真实事故**：解锁 effect 用 `useChatConvStore.getState().taskQueue` 命令式读，依赖数组里也没有它：

```ts
useEffect(() => {
  if (pendingTask?.id) return;
  const { taskQueue } = useChatConvStore.getState();   // ← 命令式读
  if (taskQueue.includes(conversationId)) return;
  lockRef.current = { locked: false };
}, [pendingTask?.id, rewardOpen, conversationId, lockRef]);   // ← 没有 taskQueue
```

首次运行时若 `taskQueue` 恰好还含本会话就 return；之后服务端把它摘掉了，**effect 不再重跑** → 锁永远解不开
（表现为「任务早完成了却一直提示 Please handle this message first」）。

**判据与触发源脱节** —— effect 只在**别的**依赖变化时才「顺便」重新判断，判据本身的变化它看不见。

> **`getState()` 命令式读 + 依赖数组遗漏 = 这个判据永远不会主动触发重算。**
> 要么订阅它并放进 deps，要么明确这个值在 effect 生命周期内不会变。

### 附带一条：`ref` 放进依赖数组毫无作用

`lockRef` 写在 deps 里不会有任何效果（引用恒定，`.current` 变化不触发重渲染），
但**容易造成「我已经处理了依赖」的错觉**。上面那个 effect 就是这么骗过自己的。

## 「一次性快照 + 早退」是异步补数据场景的通病（2026-07-20）

```ts
const others = taskQueue.filter((id) => id !== convId && id in chatMap);
if (others.length === 0) return;   // ← 错过这一次就永远没有第二次
```

问题：`chatMap` 正在被异步填充（`await IMManager.getConversation()` 逐个补），
调用那一刻目标还没进去 → 直接放弃 → **按钮不出、兜底定时器也不起，而它只有一个调用点**。

**修法：改成响应式重试，而不是赌时机。** 置一个 `awaiting` 标记，由 effect 跟着「已就位的数量」重算，
目标到位后自动补做；顺便注意新事件到达时要把标记一起清掉。

> 凡是「取当前列表 → 空就 return」的逻辑，都要问：
> **这个列表是不是正在被异步填充？错过这一次还有没有第二次机会？**

## 验证方法（无浏览器也能做）

Node 里模拟 effect 的重挂语义 + 父组件每 1s 触发：

| | 3s 后淡出 | 3s 后关闭 | effect 重挂次数 |
|---|---|---|---|
| 旧 `deps=[open, onClose]` | ✗ | ✗ | 3 |
| 新 `deps=[open]` + ref | ✓ | ✓ | 1 |

## 相关

- 落地：PR #571，commit `21269e1c7`
- 同源的「回调/锁 与 React 生命周期」坑：[[uselockfn-swallows-gesture-terminal]]
