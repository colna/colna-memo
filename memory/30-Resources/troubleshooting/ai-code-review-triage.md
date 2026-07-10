---
title: AI code review 意见分诊：先验「触发前提是否可达」
date: 2026-07-09
tags: [troubleshooting, code-review, react, sitin-next, 方法论]
---

# AI review 的九条意见里，只有一条是真的

PR #571（app-pwa 加钱弹窗）两轮 AI review，共 9 条「问题」。逐条查证后：**1 条真、8 条不成立**。
而且两轮 review 都判「可以合并」，把唯一真 bug 标成了 🟡 Important + 「低概率」。

## 唯一真的那条 —— 且比 review 描述的严重

> review：「快速连续触发两次奖励时，`fading` 可能有一帧中间态，概率低。」

真相：**第一枚金币之后，每一枚都错。**

```tsx
// 父组件无条件渲染
<RewardPopup open={rewardPopup.open} ... />

// 组件内
if (!open) return null;      // ← 只是不渲染内容，组件实例【不卸载】
```

`return null` ≠ 卸载。`fading` 在第一次淡出后一直是 `true` → 第二次奖励以 `opacity:0` 上屏、
按 `transition` 淡入 400ms，`ep-pop` 的弹入动画在一张看不见的卡片后面播完了。

修法：`open` 落下时同步 `setFading(false)`。

> **教训：review 把 bug 归为「低概率」时，自己复算一遍。**

## 八条不成立的，全是同一个模式

**把「理论上存在的 API 缺口」当 bug，不验证触发前提在这份代码里是否可达。**

| review 说 | 为何不成立 |
|---|---|
| `var(--chat-*)` 可能未定义，需加 fallback | 四个变量都在 `:root`（`@layer base`），无主题切换，不存在缺失路径 |
| `FADE_MS <= 0` 需断言 | 给不可能发生的情况写防御 |
| `--dx/--dy` 的 `as CSSProperties` 需全局 d.ts 扩展 | 类型噪音换类型噪音，不改行为 |
| `key={i}` 触发 lint | lint 实际通过；`FLECKS` 是静态常量 |
| `resize` 覆盖不了 iOS 键盘 → 手指偏移 | `wantHand` 含 `isVoice`；文本框**只在 text 分支渲染**，切 voice 即卸载 → 键盘与手不可能共存 |
| 改用 `ResizeObserver` 观察容器 | **技术上就错**：RO 只观察**尺寸**变化，不观察**位置**。元素平移而尺寸不变时一次都不触发 |
| portal 在组件卸载时有残影 | React 卸载时 portal 子树在**同一次 commit** 移除，没有多余帧 |
| cleanup 里补 `setHandAt(null)` | 卸载路径上是 no-op（组件已经没了） |

## 分诊流程

1. **先问「触发前提可达吗」**，再看建议。多数假阳性死在这一步。
   - 变量真的可能未定义吗？→ grep `:root`
   - 那两个状态真的能共存吗？→ 看它们的渲染分支
2. **建议本身也要审**。`ResizeObserver` 那条方向就错了；照做等于加一个永远不触发的监听。
3. **被标成「低概率 / Minor」的，反而要复算。** 真 bug 藏在这里。
4. **拒绝的理由写进 PR body**，reviewer（和未来的自己）才知道不是漏看了。

> 照单全收的代价：往代码里塞八处无用防御 + 一个错误的 `ResizeObserver`，
> 而真正的那条 bug 仍被当成「低概率」放过。

## 相关

- 落地：PR #571（merge commit `aac2f1d0e`）
- [[react-effect-timer-callback-dep]]（同批修的另一个 React 生命周期坑）
