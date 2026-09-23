---
title: Luka 骨架屏 shimmer 偶发黑亮竖带（录屏可见）
date: 2026-09-23
tags: [react-native, expo-linear-gradient, luka, troubleshooting]
---

# 现象

打开他人资料页、`ProfileSkeleton` 在场的那 ~1s 里，屏幕出现一条**全高、硬边、内部黑→灰→黑**的竖带，从左向右匀速扫过（录像里像一根「黑亮柱子」）。用户 20:36 的 `.qt` 录屏（`飞书20260923-203607.qt`）里拍到。

# 定位（直接从 .qt 量）

- `ffmpeg -vf scale=540:1` 抽「整屏按行平均后的一行」亮度，逐帧找暗带边缘：**26px/帧 @30fps ≈ 563pt/s**，带宽 ≈251px ≈ **181pt**。
- 与 `apps/luka/src/components/xuanyu/skeleton-shimmer.tsx` 对上：`SHIMMER_WIDTH=176`、`useShimmer` = `translateX: -176 → width(390)`、`duration: 1000` ≈ **566pt/s、176pt 宽、1s 一圈** —— 三个数全吻合。
- 时间上也只在 1.1–2.0s（`ProfileSkeleton` 存活的那一秒），带子右边缘扫出屏，恰好数据到位、骨架卸载。
- 带内亮度剖面 = 该渐变 alpha 坡（0 → 0.8 → 0）逐点吻合：带中心实测 211 ≈ 0.8×255=204；84% 处实测 64 ≈ 0.8×0.32×255=65。
- 对照组：同日 13:52 的录屏里 Chats 骨架（同一个 `ShimmerOverlay`）正常（扫光是压在浅底上几乎看不见的白色洗），说明**偶发**，不是必现样式。

# 根因判断

正常情况下这是一道透明白洗，压在奶油底上本就看不见。出问题时透明停靠点被当成**不透明**画：`rgba(255,255,255,0)` 以 premultiplied RGB（= 纯黑）落进不透明位图，`rgba(255,255,255,0.8)` 落成灰 —— 白洗就变成黑亮竖带。渲染路径是 iOS 的 `expo-linear-gradient`：`LinearGradientLayer.display()` 用已废弃的 `UIGraphicsBeginImageContextWithOptions` 把渐变 CPU 光栅成位图（`node_modules/expo-linear-gradient/ios/LinearGradientLayer.swift`），alpha 在这一次光栅里偶发丢失。上游相关：expo#47553 / #46110（同一位图路径在 iOS 26 因尺寸/时机出过崩溃）、#47554 / #48062（改成 `CAGradientLayer` 的 PR，尚未合）。

# 规避 / 修法

1. **让扫光不依赖 alpha=0 的停靠点**：改用不透明同色系色阶（`background → #FFF → background`），或「实色块 + `opacity` 动画」。即使 alpha 再出问题，也只会是「一块浅色」，不会变黑。
2. 或 patch / 升级 `expo-linear-gradient` 走上游的 `CAGradientLayer` 实现（PR 未合，需 `pnpm patch`）。
3. 复现验证：真机限速打开资料页（让骨架多停几秒）+ 屏幕录制，看黑带是否出现；若只在录屏里出现、真机屏上没有，则更偏「位图 / 录制合成」一侧。

# 排查方法（可复用）

- 手机录屏 `.qt` 用 ffmpeg 抽帧 + 每列/每行亮度剖面，量出可疑运动元素的**宽度、速度、内部斜坡**，再回代码里找同尺寸 / 同周期的动画常量 —— 本例靠 `176` 和 `1000ms` 一次锁定。
- 同族思路见 [rn-simulator-pixel-measurement.md](rn-simulator-pixel-measurement.md)（用纯色块量几何）。
