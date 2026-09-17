---
title: HeroUI/gorhom BottomSheet：面板外内容被裁 + 挂载即 open 不展开
date: 2026-09-17
tags: [troubleshooting, react-native, heroui, gorhom, bottom-sheet, luka]
---

# HeroUI/gorhom BottomSheet 的两个反直觉行为

来源：`sitin-rn` `apps/luka` 的低余额充值抽屉（low-sparks sheet）从手写 `Modal`
迁到 app 通用 `components/ui/bottom-sheet`（HeroUI `BottomSheet` → gorhom
`@gorhom/bottom-sheet` 5.2.14，2026-09-17）。两个坑都只有真机/模拟器渲染才看得见，
单测与 typecheck 完全抓不到。

## 1. 面板外的内容一律被裁，做不了「扒在上沿探头」

**症状**：设计稿里小崽从抽屉**上沿**探头（头在面板外、爪子压在边沿）。迁到通用抽屉后，
把 `PeekingHamster`（绝对定位、`top` 为负）放进面板内容里，只剩爪子下面约 10pt 的一
小条可见；再单独放一个 `top: -102` 的红色诊断方块，**整块不可见**（连本应在面板内的
10pt 也没有）。

**根因**：内容被裁在「面板顶边」这条线上。gorhom 的
`BottomSheetHostingContainer` 是绝对铺满屏、`overflow: hidden` 的容器，面板内容
（`BottomSheetView`，`position: absolute; top: 0`）与面板背景一起组成 sheet，
面板之上就再没有可绘制的空间。手写 `Modal` 时面板只是个普通 `View`，负 `top` 的
sibling 可以正常溢出并显示 —— 这是迁到通用抽屉真正丢掉的能力。

**修法/取舍**：
- 想要「探头」效果，得渲染在**面板内部**的某条边上（卡片、按钮、CTA 的上沿都可以，
  它们是普通 View）；面板自身的外沿不行。
- 或者给通用抽屉加一层**不裁切的顶部装饰槽**（提案，未做）。
- 本次把姿态换成面板内的站立抱心小崽（与签到弹层同一只）。

**验证方法（模拟器）**：临时在内容里放一个半透明诊断方块，`top` 与目标一致 →
截图看是否可见；比读 gorhom 源码快。

## 2. 挂载时 `visible=true` 不会展开抽屉

**症状**：把预览组件的初始 state 设成 `true`（或未来任何「一挂载就要打开」的用法），
抽屉**一直不出现**；改回 `false`、mount 后再 `setState(true)` 就正常。

**根因**：HeroUI 的 `BottomSheetContentContainer` 用
`prevIsOpenRef = useRef(isOpen)` 初始化。挂载时 `wasOpen === isOpen === true`，
`useEffect` 里「`isOpen && !wasOpen` → `snapToIndex`」两个分支都不命中 → 从未 snap。
只有 `false → true` 的变化才展开。

**影响面**：本仓库所有调用点都用 `visible` 控制、初始 `false`，所以产品路径不受影响；
**debug 预览/测试**想「一进去就打开」时必须用一次 `setTimeout(() => setVisible(true), 0)`
之类的翻转，不能把初始 state 设 `true`。

## 关联

- [[ios-simulator-keyboard-and-taps]]（模拟器点击/标定；窗口重开要重新标定）
- `apps/luka/src/components/ui/bottom-sheet.tsx`（组件头注释写了 Android Portal reset
  与 `enableContentPanningGesture` 的取舍）
