---
title: RN 无限滚动：触发线、内存护栏与「追加不重渲」
date: 2026-09-16
tags: [react-native, list, pagination, memo, troubleshooting]
---

# RN 无限滚动：触发线、内存护栏与「追加不重渲」

来源：2026-09-16 Luka（sitin-rn `apps/luka`）feed / nearby 瀑布流分页改版。三件事都踩过一遍。

## 坑一：滚动驱动的触发线别用「逐帧此刻可见」

- **现象**：想做「露过 2/3 张卡就预取下一页」，把「这次滚动事件里可见的卡」记入 seen 集合 —— 快速甩动时触发线可能永远到不了：`scrollEventThrottle=16` 的事件本身够密，但**曝光判定通常被节流**（每 200ms 才处理一次），两次采样之间整屏整屏穿过去的卡一张都没记上；用户甩到底，2/3 还差得远，列表就「停了」。
- **修法**：维护**视口底沿的历史最高水位**（`revealBottom = max(revealBottom, y + viewportHeight)`，在 `onScroll` 每帧更新），判「卡片顶边 < 水位」即「露过面」。水位单调、不受采样影响，回头滚也不重复触发。纯逻辑抽成 `revealedFractionReached(columns, total, revealBottom, fraction)` 放 `src/lib` 做单测。
- **配套**：`onLayout` 里也要把水位初始化成 `scrollY + height`（首屏的卡要算露过）；短列表（第一页没铺满、根本滚不动）在「列布局变化」的 effect 里补判一次，否则没有滚动事件可等。

## 坑二：分页内存护栏要「到顶即停」，不要 `slice`

- **现象**：给列表加 `MAX=120` 护栏，写法是 append 后 `.slice(0, MAX)` —— 到 120 后每个新页都被整页裁掉，**游标却继续前进**，新页一条都留不下（`freshCount>0` 也不判耗尽）：用户看到「滑到底什么都不来」，后台却一页页白拉。比真到底更糟。
- **修法**：上限检查放在**取数前**（`if (posts.length >= MAX) { exhausted = true; return; }`），并把 `merged.length >= MAX` 并入耗尽条件；`prepend` 不再裁剪。上限只是内存护栏，表现应该是「列表到底」，不是「静默吞页」。

## 坑三：「追加不影响已渲染」= 稳定 key + 卡片 memo + 回调收 item

- key 用 `item.id` 是前提，但**卡片不 memo 的话，父组件每追加一页就整片重渲**（几十上百张卡）。
- memo 要生效，props 必须稳定：把「调用处包箭头」的回调（`onLongPress={() => onMore(post)}`）改成**回调收 item 参数**（`onMore(post)` 由卡片内部调），父组件传 `useCallback` 出来的函数 —— 老卡的 props 引用不变，memo 才拦得住。
- 分页数组 append 时要保留旧元素引用（`[...before, ...page]` 后按 id 去重、**不重建对象**），`post` / `candidate` 引用才不变。
- 瀑布流装箱（每张进当前更矮的列）是确定性的，append 不会挪动已有卡的位置 —— 这是「直接跟在后面渲染」不跳位的布局前提。
