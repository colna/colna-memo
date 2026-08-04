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

## 派 agent 做 review 时的两条硬纪律（2026-07-18 / 07-20 实战补充）

上面讲的是「AI 给的意见怎么分诊」，这两条讲的是「**你派出去的 agent 本身可不可信**」。

### 一、先看 `tool_uses`，0 次 = 没干活

2026-07-18 派 2 个 fresh agent 对抗式 review PR #650（正确性 / 钱与订单两个视角）：

- 正确性 agent：**16 tool_uses**，扎实，锁定了一个真 P1
- 钱 agent：**0 tool_uses** —— 没读任何文件就返回「没发现问题」

**空转的「没发现问题」是最危险的假阴性**，尤其钱这种维度。当时钱维度我自己重查了一遍才放心。

> **纪律：拿到 agent 结果先看 `tool_uses`。个位数就要怀疑，0 次直接作废重跑。**

### 二、结论必须逐条 grep 复验，尤其这两类

2026-07-20 三轮共 9 个 agent review sitin4.0 全需求，报 90+ 条，抽验后 **3 条不成立**：

| 误报 | 根源 |
|---|---|
| 「videoTips 单价少 100 倍」，建议补 `raw=true` | agent **假设**了 `reward.cents` 的单位。用真实数据 `2000 → 20¢ → $0.2` 验算 PASS。**照改会让线上单价变 100 倍** |
| 「taskQueue 没按红黄绿排序」判 P1 | proto 注释明写 `backend pre-sorted red > yellow > green`，前端依赖后端契约是设计如此 |
| 「`pendingJumpRef` 是死代码」 | 实际有 3 处引用 |

**两类高发误报，必须验：**

1. **涉及单位 / 量纲换算的** —— agent 容易假设单位而不去查真实数据。**用一组线上真实值端到端算一遍**，比读十遍代码可靠。
2. **断言「死代码 / 零引用」的** —— 必须 `grep -rn` 出全部引用点数给我看，不接受「我看了一下没找到」。

> **产出报告时把误报单独列在最前面。** 07-20 那份 review 我把 2 条误报写进正文第零节，标题就是「不要照着改」—— 否则读者照抄改坏正确的代码，比不做 review 更糟。

## 相关

- 落地：PR #571（merge commit `aac2f1d0e`）
- [[react-effect-timer-callback-dep]]（同批修的另一个 React 生命周期坑）

## CI 基础设施:中转 524 / streaming / reasoning_effort(2026-08-03/04,sitin-next)

> 上面讲的是「评审意见质量分诊」;这节是「让 workflow 跑出评审」的基础设施坑。sitin-next 的 `.github/workflows/ai-code-review.yml`(github-script 里 fetch 中转)切到 xingsuancode `gpt-5.6-sol` 后踩的。

- **现象**:PR 上 AI Review 报 `524` + `{"error":"openai_error","type":"bad_response_status_code"}`;后来又出现「跑满 12 分钟、正文 0 字」。
- **根因链**:`524` 是 **Cloudflare idle 超时**(实测 **~125s**),来自中转**上游**(xingsuancode 边缘是 nginx,只透传状态码;nginx 超时会给 504 不是 524)。**不是**鉴权/模型问题(小请求 200 秒回)。
- **两个必须同时改的点**(缺一不可,实测):
  1. **`stream: true`**:流式下推理阶段持续吐 keep-alive 块,不断重置 idle 计时器 → 根治 524(连接能活过 234s+)。
  2. **`reasoning_effort: "low"`**:`gpt-5.6-sol` **默认推理就很重**——实测 `max`/`medium`/**不带该字段(默认)** 非流式全 524/125.5s,**只有 `low` 过**(~60s / 出内容)。
- **「正文 0 字」= stream 开了但 reasoning_effort 还是 max/默认**:流式撑住不 524,但模型把整段预算全花在「思考」(reasoning token 不算 content),流结束正文为空。**光删掉 `max` 没用(默认就重),必须主动写 `low`。**
- **github-script 流式写法**:非流式是 `response.json()`;流式要 `response.body.getReader()` 逐块解析 SSE(`data: {...}` → 累积 `choices[0].delta.content`,遇 `[DONE]` 结束),否则解析不到。心跳 `console.log` 进 Actions 日志便于观察。
- **实测数字**:stream+low → **117s / 正文 7160 字 / 审 26 文件**;stream+max → 711s / 0 字;非流式任意档 → 524@125.5s。
- **教训**:改 AI review 的模型/参数,`stream` 和 `reasoning_effort` 要**一起**改;只改注释不改值(注释写 low、值留 max)会得到「不报错但 0 字」的假成功,比 524 更隐蔽。

## proto submodule 合并/更新(sitin-next,2026-08-03)

- **合 main 遇 `business-pwa-proto/proto` submodule 冲突**:用 `git merge-base --is-ancestor A B` 判祖先——谁包含对方取谁(本次 ours release/test ⊇ main);`src/gen` 是生成代码**不信 auto-merge**,`git checkout HEAD -- src/gen` 取超集侧,再 `pnpm --filter @heyhru/business-pwa-proto build` 重建 dist。
- **给 PR「合基线」要合它的 base 分支,不是无脑合 main**:PR base=feat/sitin4.1 时误合 main,把 main 相对 base 的 152 文件全灌进 diff(44→194)。先 `gh api .../pulls/<n> --jq .base.ref` 看清 base。
- **更新 test proto**:submodule `git fetch origin release/test && git checkout <sha>` → `bash scripts/generate.sh`(protoc 在 /opt/homebrew/bin)→ `pnpm --filter business-pwa-proto build` → app-pwa `tsc -b` 验证。
