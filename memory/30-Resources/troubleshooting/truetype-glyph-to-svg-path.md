---
title: 无依赖从 TrueType 提取字形为 SVG path（RN monogram 场景）
date: 2026-10-09
tags: troubleshooting, truetype, font, svg, react-native, sitin-rn, sofia
---

# 无依赖从 TrueType 提取字形为 SVG path

场景：sofia 金币改版，要用 Archivo Black 的大写 "S" 做 monogram（react-native-svg 组件）。
本机无网络装不了 opentype.js / fontkit，改为手写 Node 解析 TTF。

## 结论（可直接复用）

脚本：`_attachments/truetype-extract-glyph.js`

```sh
node truetype-extract-glyph.js "S" /path/to/Font.ttf [targetHeight=17.6] [viewBox=48]
```

- 输出：居中缩放到 viewBox 的 SVG path，可直接进 react-native-svg 的 `<Path d>`。
- 自检：解析 bbox 与 glyf 头 `xMin/yMin/xMax/yMax` 不一致时直接报错退出。

## 解析链（最小实现）

1. 表目录 → `head`（unitsPerEm / indexToLocFormat）、`maxp`、`cmap`、`loca`、`glyf`。
2. `cmap`：format 12 (3,10) 优先，回退 format 4 (3,1)；format 4 的 `idRangeOffset`
   地址 = 该 idRangeOffset 自身地址 + rangeOffset + 2 × (code − start)。
3. `loca[gid] → loca[gid+1]` 取 glyf 字节；`numberOfContours > 0` 为简单字形。
4. flags（bit3=repeat，表示后续 repeat 个点复用该 flag；bit0=on-curve）+
   x/y 增量 → 绝对坐标。
5. 轮廓 → path：on/off 点（二次贝塞尔）；off + off 之间插中点；`Q` 命令；闭合 `Z`。

## 坑（都实际踩过）

| 坑 | 现象 | 修法 |
| --- | --- | --- |
| 坐标分批读 | 逐点交错读 x/y 不报错，形状"看起来像字"但完全错（bbox 漂移几百单位） | TrueType 先存**全部 x 增量**、再存**全部 y 增量** —— 两轮遍历 flags |
| 短增量符号 | ≥128 的短增量差 256，累积成大偏移 | 短增量读 **uint8**，按 flag bit4/bit5（same-or-positive）决定 ± —— 不要 readInt8 |
| 缺自检 | 错误形状可能"像"字形，肉眼难发现 | 断言解析 bbox == glyf 头 bbox，一次定位上面两坑 |

## 备选路径

若目标字形已在现成 SVG 大 path 里（如 app icon 的 wordmark），可按 `M` 拆子路径，
用 Playwright 渲染每个子路径截图肉眼认字。注意：sofia 的 `icon.svg` / `splash-icon`
wordmark 实际渲染出来是 **"Iris."**（从 iris 复制 app 时的品牌残留，2026-10-09 发现），
不要按注释想当然 —— 注释写的是 "Sofia"。
