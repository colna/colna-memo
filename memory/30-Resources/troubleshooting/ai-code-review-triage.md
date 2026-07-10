---
title: AI code review 意见分诊：先验「触发前提是否可达」
date: 2026-07-09
tags: [troubleshooting, code-review, react, sitin-next, 方法论]
---

# AI review 的十四条意见里，只有两条是真的

跨 PR #571 / #575 共四轮 AI review，14 条「问题」。逐条查证后：**2 条真、12 条不成立**。
每一轮都判「可以合并」，而唯一的真 bug 被标成 🟡 Important + 「低概率」。

## 战绩

| PR | 轮次 | 条数 | 真 | 假 |
|---|---|---|---|---|
| #571 | 两轮 | 9 | 1 | 8 |
| #575 | 第一轮 | 2 | 0 | 2 |
| #575 | 第二轮 | 3 | 1 | 2 |

**两条真的**：
1. 第二枚金币淡入（#571）—— review 说「低概率一帧闪烁」，实为**第一枚之后每次都错**。
2. `catch` 吞错误无日志（#575）—— 但 review 说的「完全无法定位」是夸张的（`httpClient` 已把传输层错误报进 Sentry）；
   真实缺口是「上报只带 URL，看不出是哪一次拉黑」+「白名单外的业务码完全不上报」。

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

## 第二条真的 —— 但理由和 review 说的不同

> review：「`catch` 吞掉错误，开发/运维**完全无法**从日志定位原因。」

夸张了。`httpClient` 装了 `sentryMonitor`，`HttpClient.ts:164` 对非 401 的传输层错误会 `reportApiError`（带 userId/username）。

真实缺口是另外两个，review 都没提：

- Sentry 上报只带 **URL + 状态码**，看不出是**哪一次**拉黑（无 `conversationId` / `targetUserId`）
- 不在 `errorCodesToReport` 白名单里的**业务码完全不上报** —— 「本周 6 次已用完」会毫无痕迹

所以日志要带上下文，而不是照建议只打一个 `err`。**采纳一条意见，也要先验证它的理由。**

## 十二条不成立的，全是同一个模式

**把「理论上存在的 API 缺口」当 bug，不去读被调用者的实现，也不验证触发前提是否可达。**

### PR #575（拉黑链路）

| review 说 | 为何不成立 | 一步就能证伪 |
|---|---|---|
| `deleteConversation` 可能抛，需 try/catch | 它**自己内部就有 try/catch**，永远返回 `boolean`，从不 reject | 跳到 `IMManager.ts:371` |
| `convData` 为 null → 确认弹窗显示 $0.00 | 结论对，**病因错**：不是加载延迟，是**按钮压根不该显示**（`endChatEnabled` 门被主线丢了） | 看按钮的渲染门控 |
| `convData` 进依赖 → `useLockFn` 锁失效 | `lockRef` 是 `useRef`，**挂在组件实例上，不挂 `fn` 引用**；回调重建只换 wrapper | 点开 `useLockFn` |
| 改用 `useRef` 稳定引用 / 按字段拆依赖 | 同上，锁本就有效；`convData` 引用也稳（store 直取） | 同上 |
| `clearCountdownCloud` 该进依赖数组 | 它是**模块级函数**（`index.tsx:85`，零缩进），引用永远稳定 | 跳到定义 |

> 三条都只要**跳到定义**就能证伪。review 却在猜「如果它是组件内闭包呢」「如果 store 每次返新引用呢」。

### PR #571（加钱弹窗 / 引导手）

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

0. **先跳到定义。** 「这个 async 可能 reject」→ 读它的实现；「这个引用可能不稳」→ 看它定义在哪。
   #575 里三条假阳性，全是一次 `grep`/跳转就能推翻的。
1. **再问「触发前提可达吗」**，然后才看建议。多数假阳性死在这两步。
   - 变量真的可能未定义吗？→ grep `:root`
   - 那两个状态真的能共存吗？→ 看它们的渲染分支
2. **建议本身也要审**。`ResizeObserver` 那条方向就错了；照做等于加一个永远不触发的监听。
3. **被标成「低概率 / Minor」的，反而要复算。** 真 bug 藏在这里。
4. **拒绝的理由写进 PR body**，reviewer（和未来的自己）才知道不是漏看了。

> 照单全收的代价：往代码里塞十余处无用防御 + 一个永不触发的 `ResizeObserver` + 一次没必要的依赖拆分，
> 而两条真 bug 里，一条被当成「低概率」放过，另一条的理由是错的。

## 相关

- 落地：PR #571（merge commit `aac2f1d0e`）
- [[react-effect-timer-callback-dep]]（同批修的另一个 React 生命周期坑）
