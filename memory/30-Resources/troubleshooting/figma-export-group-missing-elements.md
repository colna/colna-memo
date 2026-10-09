---
title: Figma REST 导出 group 丢子元素（PNG/SVG 均缺）
date: 2026-10-09
tags: figma, troubleshooting, export
---

# Figma REST 导出 group 丢子元素

## 现象

用 Figma REST `/v1/images?ids=<GROUP_ID>&format=png|svg` 导出某个 GROUP 节点时，产物**静默丢失部分子元素**：
- sitin 案例：导出「Co-brand lockup · Instagram ⇄ SITIN」group `9433:9915`，PNG 和 SVG 都缺中间的双向箭头（箭头由两个嵌套 group 组成，内层为 rectangle + vector 组合）；
- 同文件里结构类似的 SC 版 group（箭头是纯 vector line）导出正常。

## 根因（推断，未获官方确认）

Figma 服务端对**特定 Group 的渲染导出**存在丢失（嵌套 group / rectangle+vector 组合时更易触发）。
证据：把同一内容的父容器 Frame（`9433:9914`，纯 Frame 无变换）作为导出对象，产物完整；而 group 导出两次（1x/2x）均缺同一元素。

## 修法

1. **降一级节点导出**：导出目标从 GROUP 换成其父 FRAME（或往上一层容器 frame），产物完整。
2. **导出后自检**：SVG 产物里统计 `<path>` / `<rect>` / `<image>` 数量，与节点树子元素清单对比；PNG 则肉眼/辅助工具核对关键元素。
   ```python
   import re
   s = open("export.svg").read()
   print(len(re.findall(r"<path[^>]*>", s)), len(re.findall(r"<rect[^>]*>", s)))
   ```
3. 有多个同构节点时，逐个导出对比（结构相似但渲染路径可能不同）。

## 关联

- 使用场景：登录页合作标识素材导出（`packages/app-ins-scripts` loginCoBrandAsset）。
- Figma token 来源：`~/.claude.json` figma-developer-mcp 配置里的 `--figma-api-key`。
