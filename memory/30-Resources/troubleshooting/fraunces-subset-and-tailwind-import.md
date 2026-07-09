---
title: 字体两个静默坑：子集缺字形会逐字回退；Tailwind v4 之后的 @import 会被丢弃
date: 2026-07-09
tags: [troubleshooting, css, 字体, tailwind, vite, sitin-next, app-pwa]
---

# 两个不报错、不告警、只在真机上看得见的字体坑

## ① `@font-face` 不写 `unicode-range` = 声称拥有全部码位

缺字形时浏览器**逐字回退**到下一个 family。表现是「数字是 Fraunces、字母是 Georgia」的混排，
**不报错、不告警、构建全绿**。

踩坑经过（PR #568）：把 Fraunces 子集化成 `0-9 . , $ ¢`（2.4 KB），理由是「只用于余额和倒计时」。
后来 `--chat-display` 被用到**词**上：

```
Time's up                    taskDrawerStates.tsx
{title}                      probeDrawerParts.tsx（探针弹窗标题）
Show him you're real         bubbles/VideoTipsBubble.tsx
{s.title}                    RiskSheet.tsx
```

这些标题**从合入起就一直是 Georgia**，我从没真渲染过，headless 也验不出来。

> **规则：子集化前先 grep 该字体的所有用处**，别信「它只用于数字」这类记忆。
> `grep -rn "chat-display\|Fraunces" src`

**重做**：`fontTools.subset` 保留完整拉丁集（222 码位 / 122 字母），wght=600、opsz=19 静态实例。
2.4 KB → 17 KB（超过 Vite `assetsInlineLimit` 4096B，不再内联，变成同源请求，可接受）。

## ② Tailwind v4：`@import "tailwindcss"` 之后的任何 `@import` 都失效

```css
@import "tailwindcss";        /* ← 就地展开成【规则】 */
@import "./fonts.css";
@import url("https://fonts.googleapis.com/css2?family=Inter...");   /* ← 已非法 */
```

构建告警原文：

```
@import rules must precede all rules aside from @charset and @layer statements
```

lightningcss **直接丢弃**那一行。主线里躺了很久没人看告警 → **Inter 从加入起就没加载过**，一直回退 Saans。

> **规则：读构建告警。** CSS 的 `@import` 必须在所有规则之前；Tailwind v4 的入口 import 会展开成规则。
> 要外链字体就放**第 1 行**，或者干脆自托管。

**验证方法**：把主线原样那行放回去，`rm -rf dist && vite build`，grep 产物里有没有 `googleapis`。
不要用增量 build —— 200ms 那次是复用旧产物，`dist/` 里看到的是上一次的结果。

## 相关

- 落地：PR #568 的 merge commit `8394c3d7c`
- [[sitin4-workbench-prototypes]]（原型侧的字体约定，其中「Fraunces 只用于数字」已过时）
