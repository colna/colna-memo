---
title: RN 瀑布流滚动卡顿与 BlurView 页头（luka）
date: 2026-09-15
tags: troubleshooting, react-native, performance, expo, luka, sitin-rn
---

# RN 瀑布流滚动卡顿与 BlurView 页头

来源:`apps/luka` Discover / Nearby 改瀑布流(2026-09-14)。两件事常一起出现:浮起页头(毛玻璃)
+ 长列表。各自的坑都不在表面。

## 一、上滑到底明显卡顿:`onScroll` 每帧 `setState`

- **现象**:瀑布流滚到底明显掉帧,正好叠上滑窗口换新图那批。
- **根因**:`onScroll` 每帧 `setScrollY(...)` → 整棵树(十张卡,每张 `LinearGradient` + 照片)
  每帧重渲染。换窗口的新图解码又正好落在这时。
- **修法**:滚动只写 **ref**(`scrollY` / `viewportHeight` / `lastCheck`);曝光判定按
  `EXPOSURE_CHECK_MS = 200` 限流;`onScrollEndDrag` / `onMomentumScrollEnd` 收尾补判一次。
- **判据**:凡是 `onScroll` 里调 `setState` 的,先问它能不能进 ref。

## 二、滑窗口的闩别放在盯数据的 effect 里

`FlatList` 的 `onViewableItemsChanged` 在自绘瀑布流里没有,曝光得「滚动偏移 vs 每张卡的 y」
自己判(卡高是解析算出来的,比逐张 `onLayout` 准)。滑窗口的闩要按**内容高度变化**在
`onScroll` 里释放 —— 放进盯 `candidates` 的 effect 会被 biome 的
`useExhaustiveDependencies` 判成多余依赖(那个 effect 只在 ref 上写字,没有响应式依赖)。

## 三、BlurView 页头三个坑

1. **绝对定位参照的是边框盒**:想铺到状态栏底下,`top: -insets.top` 再配
   `paddingTop: insets.top` 会**多减一次**,标题钻到状态栏底下。`insets.top` 值本身没错
   (iPhone 17 实测 ≈59)。
2. **`BlurView` 各层是比例,父容器高是 auto 时百分比解析到整屏** → 玻璃盖满全屏。
   必须把量出来的 `height` 传下去(`onLayout` 量页头,实测 ≈108)。
3. **模拟器开发者菜单的遮罩会让全屏发灰**,一度误判成玻璃没生效 —— 用 osascript 发
   `Cmd+D` 关掉即可,别去改代码。

## 四、页头浮起时子页要让位

页头绝对定位(`top: 0, zIndex: 10`)后,内容是从它底下滚过去的:子页要按量出来的
`headerHeight` 让出顶部(deck 容器 `paddingTop`、masonry `paddingTop: topInset + 12`、
信任横幅 `paddingTop: headerHeight + 8`)。**注意别让两次**:`discover.tsx` 的 feed 子页
已经由横幅让过位,再传 `headerHeight` 会多出一整条 ≈120pt 的空白死区。
