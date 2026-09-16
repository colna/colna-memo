---
title: Metro 改了代码不热更：hot=false 与 sed 写文件
date: 2026-09-16
tags: [react-native, metro, expo, troubleshooting, luka]
---

# Metro 改了代码不热更：hot=false 与 sed 写文件

「改了代码，模拟器上没反应」的两个不同根因（2026-09-15 luka 一天里各撞一次）：

## 1. bundle 带 `hot=false` —— touch 也没用

模拟器这次拉到的 bundle URL 里带 `&hot=false`（日志可见）：
- 此时 **Fast Refresh 整体不生效**，`touch` 文件、改常量都不会更新；
- 必须 **terminate + launch**（冷启动重新拉 bundle）才能看到改动。

症状：连续几轮「改了没反应」，误以为是 Metro 监听坏了 —— 先在调试日志里看这次 bundle 是不是 `hot=false`。

## 2. `sed -i` 写文件不被 watchman 拾到

用 `sed -i` 原地替换文件后 Metro 不重建（watchman 看到的是替换后的 inode）。
- 修法：写文件用编辑器工具，或改完 `touch` 一下；
- 注意：这条只在正常（`hot=true`）时是解药，`hot=false` 时 touch 也没用，见上。

## 验收习惯

- 在跑着的模拟器上验收前，先 terminate + launch 拿新 bundle，再截图 —— 否则可能对着旧 bundle 白排查（calls-8 那次就是这样）。
