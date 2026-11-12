---
title: Lottie JSON 手写踩坑：形状层级、3D scale、首帧 easing
date: 2026-11-12
tags: [troubleshooting, lottie, animation, bodymovin]
---

# Lottie JSON 手写踩坑

背景：为 Onia 手写 IP loading Lottie（`Downloads/onia-loader/onia_loader.json`），用 lottie-web 5.12.2 逐帧验证。下面四个问题都表现为「整个角色不渲染 / 眼睛被盖住」，但 JSON 语法本身合法。

## 1. shapes 数组顺序是「先上后下」

- 现象：blob 放 `shapes` 数组第一位时，眼睛完全看不见。
- 原因：renderer 把数组靠后的 shape 先画（底层），靠前的后画（顶层）。
- 修法：眼睛（lids、whites）放前，blob 放最后。

## 2. group transform 的 scale 动画必须用 3D

- 现象：给眼睛 group 的 `s` 做 2D `[100,7]` 动画，整个 character layer 消失。
- 原因：group transform 的 animated scale 按 3D 解析；2D 数组会算出 NaN，整层不渲染。
- 修法：`[100,100,100]` → `[100,7,100]`。

## 3. 首帧 keyframe 不能只有 s

- 现象：blink 的 `t:0` 用只有 `s` 的 key，帧 0 整层消失，到 blink 帧才出现。
- 修法：首帧也要带 `i/o`；用 `{"t":0,"s":[...],"i":...,"o":...}`。

## 4. 用现代 bodymovin keyframe 格式

- 旧式 `{"t":...,"s":...,"e":...}` 配合末端 key 可能导致不渲染。
- 改成每个 key 只写 `s`，插值到下一个 key 的 `s`（参照 sitin-next 的 `cashout_success.json` / `ani.json`），lottie-web 正常。

## 验证方法

- 用 lottie-web 在 Chrome 里 `anim.goToAndStop(frame, true)` 逐帧截图（0/18/36/44/52）。
- 检查 `#loader svg path` 的 `d` 是否为空；为空说明解析失败，不是配色或尺寸问题。
