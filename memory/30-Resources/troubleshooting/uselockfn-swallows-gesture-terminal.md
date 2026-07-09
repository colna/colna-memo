---
title: useLockFn 会吞掉手势的终态调用（按住式录音卡死）
date: 2026-07-09
tags: [troubleshooting, react, sitin-next, app-pwa, 手势, 并发]
---

# `useLockFn` 是防连点，不是临界区

`packages/app-pwa/src/hooks/useLockFn.ts`：

```ts
if (lockRef.current) return;          // ← 锁住期间的调用被【静默丢弃】，不是排队
lockRef.current = true;
try { return await fn(...args); } finally { lockRef.current = false; }
```

**锁的持有时长 = 整个 async body**。body 里只要有网络请求，锁就横跨整个请求。

## 踩坑：按住式语音「松手还在录制」

`ChatVoiceRecorder.finish()` 曾是 `useLockFn(async () => { ... await stop(); ... await sendVoice() })`。

```
录第一条 → 松手 → finish 拿锁 → stop() → setRecording(false) → await sendVoice()   ← 锁没放
                                        ↓ 上传还在飞（慢网数秒）
再按住录第二条（startRecording 查的是 recording/starting，都 false → 正常开录）
松手 → finish() → lockRef 仍 true → return                    ← 这一下被吞了
```

`stop()` 没调、`recording` 没清 → 药丸停在录音态、麦克风常开、计时继续走。
之后再按也被 `if (recording) return` 挡掉，**只能切文字模式（卸载组件）才恢复**。

- 「有时候」= 网快时上传几百毫秒撞不上；连发两条语音必现。
- **判据**：第二次松手时控制台不打 `ChatVoiceRecorder finish`。

## 规则

1. **必须成对的终态转换（pointerdown/pointerup、open/close、acquire/release）绝不能放在会吞调用的锁里。**
   锁只罩住真正需要互斥的那一小段（这里是 `stop()`），网络请求放锁外 `void` 掉。
2. **手势 handler 不能读 React state 判活。** `pointerup` 比 commit 快。用 ref 做同步真源
   （`recordingRef` + `setRecordingNow()` 双写），state 只用于渲染。
3. 点击式（文字发送 `ChatInputBar.handleSend`）用 `useLockFn` 没问题 —— 它没有「必须成对」的终态，
   最坏只是发送期间点不动。**同一个 hook 在手势场景下就是 bug。**

## 定性方法：仿真，别盯着代码猜

无浏览器也能验。把两版状态机搬进 Node，用 `setTimeout` 造 grant / stop / upload 三段时序，
跑三个场景（正常 / 授权中松手 / 上传未完成就录第二条）：

| 场景 | 旧 | 新 |
|---|---|---|
| 正常录发 | 正常，发 1 | 正常，发 1 |
| 授权中松手 | 正常，发 1 | 正常，发 1 |
| **上传未完成就录第二条** | **卡住**，`finish DROPPED`，只发 1 | 正常，发 2 |

20 行仿真就把 bug 钉死了。相关：[[sitin4-workbench-prototypes]]

## 遗留

并发上传两条语音，到达顺序不保证。锁本来也只是「顺带」串行化，不是设计意图；
真要保序得单开发送队列，而不是拿手势锁去顶替。
