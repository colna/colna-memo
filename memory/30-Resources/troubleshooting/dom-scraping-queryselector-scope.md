---
title: querySelector 的选择器不以元素为作用域起点（DOM 抓取踩坑）
date: 2026-07-17
tags: [troubleshooting, dom, queryselector, scraping, app-ins-scripts, snapchat]
---

# querySelector 的选择器不以元素为作用域起点

## 坑

想「跳过含嵌套气泡的外层 li」，写了：

```js
if (li.querySelector('li div[style*="border-color"]')) continue;
```

结果**每个 li 都被跳过**，函数恒返回 0 条，线上报 `ELEMENT_NOT_FOUND: No message bubbles found` —— 但 `dom_snapshot` 里 `selector_found: true`、气泡明明都在。

## 根因

`element.querySelector(sel)` 的语义是：

> **在整个 document 上匹配 `sel`，再把结果过滤成 element 的后代。**

它**不**把 `sel` 锚定在 element 上。所以 `li div[...]` 里的 `li`，可以匹配到**这个 li 自己**（它确实是自己内部那个 div 的 li 祖先）→ 返回自己内部的气泡 → 判定为「含嵌套气泡」→ 全被跳过。

后代组合符（空格）才有这个问题；**单个复合选择器**（`div[style*="x"]`、`header`）和**选择器列表**（`video[src], img[src]`）都不受影响，因为它们不依赖祖先关系。

## 修法

三选一，优先第 1：

1. **反向从目标 `closest()` 取** —— 意图最直白，零歧义：
   ```js
   var bubbles = listEl.querySelectorAll('div[style*="border-color"]');
   for (var i = 0; i < bubbles.length; i++) {
     var li = bubbles[i].closest("li");   // 最近的 li 祖先 = 最内层 li
   }
   ```
2. **`:scope`** —— `li.querySelector(':scope li div[...]')` 把选择器锚在 li 上，行为才符合直觉。
3. **显式遍历后代** —— `li.querySelectorAll("li")` 再逐个 `querySelector('div[...]')`（内层选择器不含组合符即安全）。

## 判据速记

> `element.querySelector()` 里**只要出现空格（后代组合符）**，就问一句：选择器最左边那个 tag 会不会匹配到 element 自己或它的祖先？会 → 用 `:scope` 或 `closest()`。

## 验证方法（无 jsdom 依赖也能做）

仓库里没 jsdom 时，**别改 package.json**，在 scratchpad 里临时装来跑真 DOM 样本：

```bash
mkdir -p $SCRATCH/domtest && cd $SCRATCH/domtest
npm init -y && npm i jsdom
# 加载 dom/snapchat/chat2-page.html，新旧实现各跑一遍对比
```

`sitin-next/packages/app-ins-scripts/dom/snapchat/*.html` 存着真实页面样本，正是干这个用的。**旧实现返回 0 / 新实现返回 5**，一跑就实锤，比推理靠谱。

## 教训

这个 bug 是**推理**出根因的，方向对了；但同一个文件我已经错过一次（见 [[../../50-Daily/2026-07-17]] 里 `ID_NAME_RE` 漏 `<key>`）。**DOM 选择器逻辑一律拿真样本 + jsdom 实测**，别只靠读代码 —— 成本就是 `npm i jsdom` 一行。

相关：[[sitin-next-script-error]]、[[../../50-Daily/2026-07-17]]
