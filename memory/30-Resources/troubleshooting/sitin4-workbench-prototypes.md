---
title: SITIN 4.0 工作台原型：三份 HTML 互相不一致，不能照着一份抄
date: 2026-07-09
tags: [troubleshooting, sitin-next, app-pwa, 原型, 设计稿]
---

# SITIN 4.0 工作台原型：三份不一致清单

**真源已归档进仓库**（可直接浏览器打开跑）：

```
packages/app-pwa/docs/prototypes/
  workbench-normal-state.html    (641 行)  蓝色 / AI 自动驾驶
  workbench-yellow-copilot.html  (926 行)  琥珀 / AI Exposure Alert
  workbench-red-takeover.html   (1106 行)  红色 / Red takeover
  sitin40-probe-photo.html       (932 行)  探针 · 自拍
  sitin40-probe-voice.html       (962 行)  探针 · 语音
```

## 核心教训

**同一设计系统的三份原型，同名类的实现各不相同。照着一份抄，一定继承它的 bug。**
截至 2026-07-09 已因此踩坑 5 次。**动手前先三份逐行 diff。**

## 差异清单

| | 正常(蓝) | 黄色(琥珀) | 红色 |
|---|---|---|---|
| `.top` padding | `32 14 2` | `32 14 8` | `32 14 8` |
| `.rail` | `2 0 8` | `2 0 10` + `mb:-8` | `2 0 10` + `mb:-8` |
| 小三角类名 | `.q.sel`，`bottom:-7px`，**无动画** | `.q.cur`，`bottom:-8px`，`curin` 动画 | 同黄色 |
| 环色 | 仅 `.q.b` | `.q.b` `.q.y` | `.q.b` `.q.y` `.q.r` |
| **倒计时环** | **无**（实心） | **有** | **有** |
| `.client .v` | `margin-left:auto` | `margin-left:auto` | `margin-left:4px` |
| `.blockbtn` | 无 | 无 | **有**（`ml:auto` 推到最右） |
| FLIP 实现 | 双 `rAF` | 强制回流 `void offsetWidth` | 强制回流 |
| 输入栏 | **完全没有** | 有（键盘置灰） | 有（撤掉罐头话术） |

**哪份是对的**：`.rail` 的负 margin —— 三份里两份有，**正常态那份漏了**，导致头像与余额差 3px（见 [[../../50-Daily/2026-07-08]] commit `23207ef1`）。FLIP —— 双 `rAF` 在后台标签页不触发，元素卡死，**强制回流才对**。

## 关键语义（容易看错）

1. **环色 = 当前会话状态，不是"选中"**。非当前会话的 `.arc` 存在但 `stroke:transparent`。
   映射 app：`Green(AUTO)→蓝`、`Yellow(COPILOT)→琥珀`、`Red(BLOCK)→红`。设计说明原话：*blue is AI-handled, amber is hers*。
2. **蓝环不是进度环**。正常态注释明写 *a solid active-chat marker, **not a countdown***。
   `rotate(-90deg)` 是倒计时用法的残留。**只有黄/红才排空**：
   ```js
   C = 2π × 14.5 = 91.11
   strokeDasharray  = C
   strokeDashoffset = C × (1 - remaining/total)   // 追踪「已耗时间」
   ```
   从 12 点顺时针排空。写反（用 `remaining/total`）环会随倒计时**长出来**。
3. **`¢` 是原型的系统性记号错误**。三份都写 `¢113.32` / `+¢0.30`，两位小数配 `¢`，**比真值小 100 倍**。
   真源：`useCash().cash` 是**美元**，`totalEarnedCents` 是**整数美分**。正确约定见 `utils/money.ts`（不足 $1 → `50¢`，否则 `$12.40`）。
4. ~~**Fraunces 只用于「钱」和「倒计时」**这类展示型数字~~ **← 已过时**。
   `--chat-display` 现在也用在**词**上：`Time's up`、探针弹窗标题、`Show him you're real`、RiskSheet 标题。
   所以字体子集**必须含完整拉丁集**（见 [[fraunces-subset-and-tailwind-import]]）。
   正文声明的是 Inter，但那个 Google `@import` 被构建丢弃，实际回退 Saans。
5. **探针语音的审核在 `Stop` 时触发，不是 `Send`**。通过后才出现 Play / Re-record 与 `Send — ¢0.80 earned!`；
   失败直接 `startRecording()` 重录，不回 idle。真正把媒体塞进 chat 的是**通过态那一下 CTA 点击**（`goStep4`），
   不是审核回调 —— 两份探针原型都如此。

## `.blockbtn`：同名类跨原型不同值（第 7 次）

| | `workbench-red-takeover.html` | `sitin40-inputbar.html`（**新，以此为准**） |
|---|---|---|
| 背景 | `none`（幽灵） | `var(--raised-2)`（填充灰） |
| 圆角 | `999px`（胶囊） | `6px`（圆角矩形） |
| 字重 | `500` | `600` |
| 颜色 | `var(--faint)` | `var(--muted)` |
| 内边距 | `3px 9px` | `5px 10px` |
| 字距 | — | `.02em` |
| **文案** | `Block` | `Blocking` |

> 同一份 `sitin40-inputbar.html` 里 paneA 是 `Blocking`、paneB 是 `Block`——**连它自己都不统一**。
> 用户指定 `#blockBtn`（paneA）→ `Blocking`。
>
> 三个色变量在 app 已有同值 `--chat-*` token（`--chat-raised-2` / `--chat-muted` / `--chat-rule`），直接引 token，别再硬编码 oklch。

## 色板：同名变量跨原型不同值（第 6 次踩坑的根源）

| | 探针 `sitin40-probe-*.html` | 输入栏 `sitin40-inputbar.html` |
|---|---|---|
| `--red` | `oklch(0.58 0.2 22)` | `oklch(0.5 0.19 25)` |
| `--red-dim` | `oklch(0.58 0.2 22 / .1)` | `oklch(0.5 0.19 25 / .1)` |

**且输入栏自身不自洽**：`.rec-target.hot` 的光晕写死 `oklch(0.58 0.2 22 / .15)`，与它的 `--red` 不同色。
**照抄字面量，不要「统一成 --red」。**

> 规则：**跨原型搬色板前，先 diff 两份的 `:root{--*}`**。别凭「上一份原型的红」写常量。

## 陷阱：`.rec-target` 的 `bottom` 不是相对 `.phone`

```js
recTarget.style.bottom = (phoneRect.bottom - btnRect.top + 4) + 'px';   // 像「桶底距药丸顶 4px」
```

祖先链是 `phone > convo > pane`，而 `.pane{position:absolute;inset:0}` → **offsetParent 是 `.pane`**。
`.picker` 在 pane 内、`.tabs`(64px) 在 pane 外 ⇒ pane 底边 = 输入栏底边。

**真实渲染：桶底 = 药丸顶 − 68px**（`tabs 64 + 4`）。照抄成 4px 会让手指刚离开药丸就触发取消（上滑仅 10px），
正确值让上滑约 74px。

> **`top`/`bottom` 永远相对 offsetParent。** 看到 `el.style.bottom = <另一个元素的 rect>` 时，
> 先把祖先链上第一个 `position != static` 的元素找出来，再算。

## 方法论

- **收到原型第一件事：`cp` 到 scratchpad + 归档进仓库**。飞书临时目录会清文件（被坑 2 次）。
- **动手实现原型动效前，先 `grep "@keyframes" src/styles/`**。
  主线 `ac1684e02` 早把加钱弹窗的 `ep-pop`/`ep-flip`/`ep-glint`/`ep-rise`/`ep-burst` 铺好了，逐帧与原型相同，**零引用**。
  差点又写一套。同理，顶栏余额滚动 `bal-roll-in/out` 在 PR #565 就已实现 —— **先查再写，两次省掉重复劳动**。
- **原型的 `prefers-reduced-motion` 块可能漏项**。加钱弹窗的 glint 用 `animation`，而原型只写了 `*{transition:none!important}`，管不到它。
  一道扫过金币的光对要求减弱动效的用户仍是动效 —— 有意偏离原型，补进降级块。
- **凡是 `prefers-reduced-motion` 要覆盖的属性，绝不能写进内联 `style`**（内联优先级更高）。
  动画的起始态靠 0% 关键帧（无 delay）或 `backwards` 填充，不要内联种子值 ——
  否则降级用户看到的是永远停在 `scale(0.6); opacity:0` 的卡片，即「什么都没有」。
- **反常的结论先怀疑前提**。我把「一离开药丸就取消」写进笔记当「原型固有行为」，根因只是 offsetParent 认错了。手感明显不对的东西，多半是自己读错了，不是设计如此。
- **类名要 grep 出来再用**。找底部导航时我 grep `tabbar`，原型里叫 `.tabs`，于是误判「原型没有 tab 栏」，进而算错了整套几何。
- **注入 class 量 computed style，两个静默失效点**（都会让你读到「上一个状态」的值，误判成规则不生效）：
  1. 原型自带的 demo `<script>` 会清掉注入的 class → 先剥掉所有 `<script>`。
  2. 直接在 `id="x"` 后追加 `class="..."` 会造成**重复 class 属性**，HTML 解析只认第一个 → 必须替换已有的 `class="..."`。
- **没有浏览器也能验**：解析原型 CSS 的 `--*` 变量与规则，对拍源码常量，写成断言脚本。比目测可靠，且不依赖 Chrome。
- **判断交互必须读 JS**。CSS 只告诉你静态态；动态文案与显隐都在 JS 里
  （典型：`#voiceLbl` 录音时被换成计时器，我只读 CSS 就断言「没有计时器」，翻车）。
- **量尺寸用注入脚本读 computed style**，别靠像素目测。
- **量「看起来居中」要量墨迹中心**，不是 `getBoundingClientRect` 的盒子中心。
- **枚举状态要从 JS 取全集**，别凭 UI 想象补：`grep "shBody.textContent ="` / `"ctaBtn.textContent ="` 把所有文案态列出来。
  （我给探针语音编了一个原型里不存在的「校验前试听」态，被用户当场戳破。）
- **`nowrap` 的行必须按真机宽度实测**。原型固定 384px 画布会掩盖窄屏溢出：
  探针自拍的三个通过 pill 在 375px(SE) 溢出 4px、360px 溢出 11px、320px 溢出 31px。
- **`@media (prefers-reduced-motion:reduce)` 里的 `animation:none` 是无障碍降级**，不代表原型没动效。
- **原型里的「相机」多半是假的**。探针自拍的全屏取景器无 `getUserMedia`、无 `<video>`，别照着造。

## 相关

- 落地 PR：#565（顶栏 + 会话头 + 倒计时环）—— **已 merge**，merge commit `e61c6fa9`
- 探针弹窗 + 输入栏按住式/上滑取消：PR #568 —— **已 merge**，merge commit `80616272a`
  （含行为变更：审核时机、onSent 时机；以及 pointercancel 一律丢弃）
- 输入栏（点击式，后被 #568 回退为按住式）：PR #559
- 加钱弹窗 + 引导手：PR #571 —— **已 merge**，merge commit `aac2f1d0e`
- 手势锁的坑：[[uselockfn-swallows-gesture-terminal]]
- review 分诊：[[ai-code-review-triage]]
- 字体的两个静默坑：[[fraunces-subset-and-tailwind-import]]
- 详见 [[../../50-Daily/2026-07-08]]
