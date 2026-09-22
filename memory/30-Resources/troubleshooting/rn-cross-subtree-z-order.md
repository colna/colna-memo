---
title: RN 里 zIndex 只在兄弟间比较 —— 跨子树的层级必须搬到同一个父节点
date: 2026-09-22
tags: [react-native, layout, z-index, luka, discovery]
---

# RN 里 zIndex 只在兄弟间比较 —— 跨子树的层级必须搬到同一个父节点

**症状**：Luka 划卡（`apps/luka/src/components/discovery/`）里，卡角那只小崽（`ParkPassHamster`，卡片之上的浮层）把卡面「名字 / 职业 · 城市」那一行的尾巴吃掉一截（连省略号都看不见）。产品要求「文字加毛玻璃底、改在小崽上面」。

**为什么不能靠调 zIndex 解决**：信息带是卡片（`CandidateCard`）的子节点，小崽是卡片**之上**的兄弟浮层。RN 的 zIndex（iOS `zPosition` / Android 子视图重排）只在**同一个父节点的子节点之间**生效，子树的绘制顺序跨不出去 —— 卡片子树里的任何 zIndex 都压不过卡外的兄弟。想跨，只能把要排前后的两层**搬到同一个父节点**下。

**修法（可复用的形状）**：让容器提供一个渲染进「卡框」的接缝，并把卡片的动画句柄一起交出去：

```tsx
// card-stack.tsx
renderChrome?: (index: number, frontStyle: AnimatedStyle<ViewStyle>) => ReactNode; // frontStyle = 正面卡的位移动画

<View pointerEvents="none" style={[StyleSheet.absoluteFill, { zIndex: 2 }]}>
  {renderChrome(index, slotStyle[front])}
</View>
```

调用方在这一层里**按文档顺序**排前后：先画小崽（不套 `frontStyle`，屏级装饰、不跟卡移动），再画信息带（套 `frontStyle`，拖动 / 划走跟着卡）。要点：

- 这一层的 zIndex 必须高过正面槽的 1（背面 0 / 正面 1 是两张卡的排序），否则整层被卡盖住。
- 整层 `pointerEvents="none"`：卡片点按与刮层手势要能从它下面收得到。
- 从别的组件传下来的 `AnimatedStyle` 直接套在 `Animated.View` 上即可，不需要把 `useAnimatedStyle` 搬进调用方。
- 不动的那件别套动画；动的那件一定要套，否则拖动时会看着像「贴在屏幕上」而不是贴在卡上。

**同源坑**：信息带原来靠卡片的 `overflow: hidden` 裁底角，搬出来以后要自己给 `borderBottomLeftRadius` / `borderBottomRightRadius` + `overflow: hidden`，否则卡底两角会露出照片的方角。

**验证方式**：本机只装了 `simctl`、没有 Simulator.app，模拟器界面点不了（截图能取、点不动）→ 这类层级改动只能靠实机截图验收。结构性判断记住「zIndex 不出兄弟层」这一条，能省一轮往返。

**后续（2026-09-22 当天）**：产品看完那一版后直接决定**撤掉卡角那只小崽**，于是整段 `renderChrome` 接缝回退、信息带回到卡片子树里（只保留毛玻璃底）。也就是说**这条经验没有被长期使用** —— 但约束本身没变（下次真有「卡内元素要压卡外浮层」的需求，仍按上面这个形状做），而且「先算清能不能靠调层级解决、不能就让产品在层级与减法之间选」这一步很值：一次往返就定案。
