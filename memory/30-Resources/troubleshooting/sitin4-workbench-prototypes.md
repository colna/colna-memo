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
4. **Fraunces 只用于「钱」和「倒计时」**这类展示型数字（`--display` 变量），正文一律 Inter。
5. **探针语音的审核在 `Stop` 时触发，不是 `Send`**。通过后才出现 Play / Re-record 与 `Send — ¢0.80 earned!`；
   失败直接 `startRecording()` 重录，不回 idle。真正把媒体塞进 chat 的是**通过态那一下 CTA 点击**（`goStep4`），
   不是审核回调 —— 两份探针原型都如此。

## 方法论

- **收到原型第一件事：`cp` 到 scratchpad + 归档进仓库**。飞书临时目录会清文件（被坑 2 次）。
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
- 探针弹窗落地：PR #568（含行为变更：审核时机、onSent 时机）
- 输入栏落地：PR #559
- 详见 [[../../50-Daily/2026-07-08]]
