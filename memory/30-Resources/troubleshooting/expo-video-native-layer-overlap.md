---
title: expo-video 的原生图层压不住：裁自己的容器
date: 2026-09-16
tags: [react-native, expo-video, troubleshooting, luka]
---

# expo-video 的原生图层压不住：裁自己的容器

场景：想让小崽「从卡片后探头」——`VideoView`（`expo-video`，播放带 alpha 的 mov）放在卡片里，结果仓鼠整只画在卡片和文字之上，把卡片里的数字都挡住了。

试过无效：提高 z-index、负外边距、调兄弟顺序 —— 都不行。

**根因**：`expo-video` 的 `VideoView` 是原生图层，不参与 RN 的视图绘制顺序，兄弟节点压不住它。

**正解**：裁自己的容器 —— 给视频包一层 `overflow-hidden`，高度少给要「被遮住」的那一截（如 `PET_PEEK = 28`），让容器边界替卡片完成遮挡。主屏环上那只探头崽不需要裁（它本来就该盖住环）时不要加。
