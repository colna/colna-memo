---
title: 分析 Chrome 性能文件（.heaptimeline / Trace .json.gz）
date: 2026-07-21
tags: [troubleshooting, performance, chrome, devtools, profiling, app-pwa]
---

# 分析 Chrome 性能文件

用户把 DevTools 导出的文件丢过来时，怎么在命令行里读出结论。两种文件解决的是**不同**问题，先别选错。

| 症状 | 该录什么 | 别录什么 |
|---|---|---|
| **掉帧 / 卡顿 / 交互慢** | **Performance trace** | Memory 堆快照（测不出渲染问题） |
| 内存涨、越用越卡、疑似泄漏 | Memory → Heap snapshot / Allocation timeline | — |

> 2026-07-21 实例：为查「输入框随键盘升起掉帧」先收到一份 `.heaptimeline` —— 它证明了没有泄漏，但对掉帧本身几乎没有信息量。换 trace 才定位到问题。

---

## 一、`.heaptimeline` / `.heapsnapshot`

就是 JSON。结构：

```
snapshot.meta.node_fields = [type, name, id, self_size, edge_count, detachedness]
nodes    扁平数组，每 6 个一组
strings  name 是到这里的索引
samples  [timestamp_us, last_assigned_id, ...] 交替
```

Python 直接 `json.load` 即可（62MB / 79 万节点约占几百 MB 内存，可接受）。

### 三个最有信息量的聚合

1. **按 `type` 聚合 `self_size`** —— 一眼看出大头是 native / string / code 还是 object
2. **`detachedness != 0` 的节点** —— **DOM 泄漏的直接证据**，同时打印它们的 name 就知道是哪些元素没被回收
3. **`samples` 相邻点求差** —— `Δid / Δt` 就是分配速率，能找出分配尖峰所在的秒

### ⭐ 读数时先扣掉 dev 环境的账

实测一份 app-pwa 的 dev 快照：**总堆 103 MB，其中 `system / ExternalStringData` 占 53 MB（52%）**。那是 vite dev 逐模块加载的**源码字符串**，生产构建后不存在。同理 `code` 段 13.9 MB、以及 protobuf 那 1,200 多个 message 的 `encode`/`decode`/`fromJSON`/`fromPartial` 闭包各 1,200 份 —— 全是 dev 无 tree-shaking 的产物。

**不扣掉这部分，会把「dev 开销」误读成「应用臃肿」。**

判断泄漏看这两个就够：`detached` 节点数（该例 254 个 / 0.04 MB → 干净）、`FiberNode` 数量（该例 179 个 → 组件树很小）。

### 一个必看的元数据

**`trace_function_infos: 0` 表示录制时没勾 "Record allocation stacks"** —— 那就拿不到「谁在分配」的调用栈，只能看总量，无法归因。要归因必须重录并勾上。

---

## 二、Performance trace（`.json.gz`）

`gunzip` 后是 `{"traceEvents": [...]}`。事件是扁平的，`ts`/`dur` 单位微秒，靠时间区间套嵌来判断父子。

### 分析顺序（从粗到细）

**1. 先排除重排** —— 统计这几个事件的次数与总耗时：

```
Layout            ← 重排，最贵
UpdateLayoutTree  ← 样式重算
Paint / PrePaint / Commit / HitTest
```

实例：17 秒录制里 `Layout` 只有 **21 次 / 6.5ms** → 直接排除「布局属性动画」这一大类。（对照：同一页面早前用 `margin-bottom` 做键盘让位时，光键盘弹起 600ms 内就有 11 次。）

**2. 找长任务** —— `name == "RunTask"` 且 `dur > 16000`(µs)。按耗时排序，记下它们的时间点。

**3. 定位用户交互的真实时刻** —— 过滤 `EventDispatch`，取 `args.data.type` 属于 `pointerdown/touchstart/click/focus/keydown/…` 的，打成时间线。**这一步最关键**：长任务清单里排前面的往往是页面加载，跟用户抱怨的交互毫无关系。

**4. 只深挖交互窗口** —— 取交互时刻 ±0.5s 的所有带 `dur` 的事件，按 `name` 聚合总耗时；再单独把 `FunctionCall` / `v8.callFunction` / `TimerFire` 的 `args.data.functionName` + `url` 聚合出来，就能看到**具体是哪个文件的哪个函数**。

### ⭐⭐ 必须先扣掉的两类噪声

**DevTools 调试器本身**。开着面板录，trace 里会塞满：

```
v8::Debugger::AsyncTaskRun     18,037 次   ← 全 trace 第二多的事件
V8Console::runTask              5,074 次
StubScriptCatchup / ScriptCatchup
```

实例中一个 **581ms 的巨型任务，内容全是 `StubScriptCatchup` x419** —— 纯调试器开销。另一个 247ms 的任务是 `v8.evaluateModule`（vite dev 逐模块求值）+ `MinorGC`。**这两个都在页面加载期，跟交互无关，却会排在长任务榜首把人带偏。**

**vite dev 的模块加载**。同上，生产构建后消失。

> 正确姿势：`pnpm build && pnpm preview` 起生产包，用 Performance 的「重新加载并录制」，**录制期间不要开着面板**，录完再打开看。

### 实例结论（可作模板）

交互窗口深挖后得到的真实构成：

| 来源 | 耗时 |
|---|---|
| `_workerSocket.onmessage @ @tencentcloud/chat` | **94.7 ms** ← 真凶 |
| GC（MinorGC 86.6 + 后台 scavenge 144.5） | 随之而来 |
| React `dispatchDiscreteEvent` | 26.5 ms |
| 埋点 SDK `collect-rangers` | 3.7 ms |

即：**卡顿与键盘动画无关，是 IM SDK 在主线程同步解析消息 + 随之而来的 GC**；它只是恰好和键盘动画撞在一起。

---

## 方法论

1. **先确认工具选对了。** 掉帧用 trace，内存用 heap。拿堆快照查掉帧，最好的结果也只是「排除了内存原因」。
2. **长任务榜首常常是噪声。** 一定要先用 `EventDispatch` 定位用户交互的真实时刻，再去看那个窗口 —— 而不是从最长的任务开始查。
3. **dev 环境的账要单独记。** DevTools 调试器、vite 逐模块加载、无 tree-shaking 的代码与字符串，能占到堆的一半、长任务的全部。
4. **数机制，别数体感。** Layout 次数、长任务时长、事件耗时都是硬数字；「感觉不丝滑」不是。参见 [[mobile-keyboard-and-viewport]] 里用 rAF 帧间隔测不出问题、换数 Layout 才一眼看到 11 vs 0 的那次。

相关：[[mobile-keyboard-and-viewport]] · [[verification-discipline]]
