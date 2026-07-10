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

## 验证方法（无浏览器也能做）

Node 里模拟 effect 的重挂语义 + 父组件每 1s 触发：

| | 3s 后淡出 | 3s 后关闭 | effect 重挂次数 |
|---|---|---|---|
| 旧 `deps=[open, onClose]` | ✗ | ✗ | 3 |
| 新 `deps=[open]` + ref | ✓ | ✓ | 1 |

## 相关

- 落地：PR #571，commit `21269e1c7`
- 同源的「回调/锁 与 React 生命周期」坑：[[uselockfn-swallows-gesture-terminal]]
