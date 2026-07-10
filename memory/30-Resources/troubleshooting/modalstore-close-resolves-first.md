---
title: modalStore.close() 会触发 onClose —— 先 close 后 resolve 会把答案改掉
date: 2026-07-09
tags: [troubleshooting, react, promise, sitin-next, app-pwa]
---

# 「点确认」和「点取消」行为完全一样，且静默无提示

## 症状

拉黑弹窗点 `Block` 什么都不发生，点 `Cancel` 也什么都不发生。没有报错、没有 toast、没有网络请求。
**线上从未成功拉黑过一次**，因为两个按钮等价。

## 机制

`stores/modalStore.ts` 的 `close(id)` **主动调用**该弹窗注册的 `onClose`
（这是有意设计：让「等待关闭」的 Promise 能 resolve，注释里写了）：

```ts
close: (id) => set((state) => {
  const modal = state.modals.find((m) => m.id === id);
  modal?.onClose?.();          // ← 这里
  return { modals: state.modals.filter((m) => m.id !== id) };
}),
```

而调用方**先 close 后 resolve**：

```ts
const close = (result) => { store.close(ID); resolve(result); };   // ← 顺序错了
store.open(ID, <...>, { onClose: () => resolve(false) });          // close 会触发它
```

`close(true)` → `store.close()` → `onClose()` → **`resolve(false)` 先落地** →
后面的 `resolve(true)` 被 Promise 忽略（Promise 只认第一次 settle）。

⇒ `confirmed` 恒为 `false` ⇒ `if (!confirmed) return` ⇒ 请求从未发出。

## 修法

**先 settle，再 close**，并用守卫让 store 回调的那次变成空操作：

```ts
let settled = false;
const settle = (result: boolean) => {
  if (settled) return;
  settled = true;
  resolve(result);
};
const close = (result: boolean) => {
  settle(result);              // 先给答案
  store.close(ID);             // 再关；它触发的 onClose → settle 已被守卫吞掉
};
store.open(ID, <...>, { onClose: () => settle(false) });
```

## 规则

1. **把 Promise 包在回调式弹窗外面时，`resolve` 必须先于任何会触发回调的操作。**
2. **只有一个 settle 入口**，用 `settled` 布尔守卫。多处 `resolve` 是味道。
3. **Promise 只认第一次 settle，后续静默忽略** —— 这就是为什么这类 bug 不报错、极难发现。
4. 拿到「点确认无反应」这类报告，**先查 Promise 是不是已经被别的路径 settle 过了**，
   再去怀疑网络层。我一开始去 mock 接口，结果那个接口压根没人调。

## 排查方法

Node 里复刻 store 的 close 语义，20 行跑三条路径：

| | 旧 | 新 |
|---|---|---|
| 点确认 | `false` ✗ | `true` ✓ |
| 点取消 | `false` | `false` |
| 点遮罩 | `false` | `false` |

## 相关

- 落地：PR #574，commit `6879ffb9d`；坏代码来自 `69df938c7 feat(pwa): chat footer`
- 全库扫过，其余 `modalStore` 使用者（`useInsTaskInit` / `anchorLoginGuard` …）没有
  「`close()` 里 resolve 的值与 `options.onClose` 不同」这种写法，只此一处。
- 同类「该发生的事永不发生，且不报错」：[[react-effect-timer-callback-dep]]、[[uselockfn-swallows-gesture-terminal]]
