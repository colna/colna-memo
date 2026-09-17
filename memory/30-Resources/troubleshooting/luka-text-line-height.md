---
title: Luka 文本行高 —— tailwind-merge 会丢掉 text-base 的 1.5 行高
date: 2026-09-17
tags: [troubleshooting, luka, react-native, tailwind, figma-write-bridge]
---

# Luka 文本行高

## 坑

按 Figma mock 解算竖向布局（`flex-1` 撑开的列）时，不确定 `components/ui/text.tsx` 的 variant `body`（= `text-base text-foreground`）会不会把 Tailwind 的 1.5 行高带到 `className="text-[22px]"` 上 —— 1.5×22=33 与 1.364×22=30 差 3pt，会把整列的位置挪几 pt。

## 根因

`cn()`（`apps/luka/src/lib/utils.ts`）是 clsx + **tailwind-merge**：`text-base` 与 `text-[22px]` 同属 font-size 分组，
后者（className 在后）胜出，`text-base` 被整条删掉 —— 连带它的
`line-height: var(--text-base--line-height)`（= `calc(1.5/1)`）一起没了，于是 RN 用字体自身的行高。

（反证：若没有 twMerge，Uniwind 的 native store 会把 unitless line-height < 6 乘上 fontSize —— `store.js` 的
`if (result.lineHeight !== void 0 && result.lineHeight < 6) result.lineHeight *= result.fontSize`，那 14px 就是 21 而不是 19。）

## 判据 / 修法

- 没有 `leading-[Npx]` 的 `text-[Npx]` → 行高 = **Nunito 自然行高 ≈ 1.364×N**（hhea ascent 1011 + descent 353，unitsPerEm 1000）：
  22 → 30、14 → 19、13 → 18。
- 代码里写了 `leading-[Npx]` 的照它取（47 屏 15/19、14/18；56 屏 16/21、13/18）。
- 快速校验 twMerge 行为：`node -e "import('tailwind-merge/dist/bundle-mjs.mjs').then(({twMerge})=>console.log(twMerge('text-base text-foreground','mt-3 text-[22px]')))"`
  → `mt-3 text-[22px] text-foreground`（`text-base` 消失）。

用于 57 屏（亲密度主页）的竖向解算：名字 22/30、天数 14/19、按钮标签 13/18。

## 补充（2026-09-17 15:xx，58/59 Debug Tools 屏实测）

从**真机截图逐像素反解**（1206×2622 = 402×874 @3x）出一层更硬的规律：无 `leading-` 时 RN 会把字体自然行高**对齐到物理像素栅格**，不是简单按 1.364×N 取整。

- Nunito Bold 17：CoreText 自然行高 23.188（ascent 17.187 + descent 6.001）→ 3x 下 69.56px → 行盒 **23.333**（70px）。
  判据：`DebugSwitchRow`（`py-3` 12×2 + 标题 + `mt-0.5` 2 + 13/17 副标题）的行距在截图里是**连续 3 段精确 199px** = 66.333pt，且五颗 `Toggle` 的中心逐一对上；用 23.19 会出 198.6px。
  带副标题的行：一行 66.33 / 两行 83.33（`12 + 23.33 + 2 + 17×n + 12`）。
- 等宽标签同理按 3x 取整：Space Mono 11 → 16.33（49px，`DebugInlineRow` 46 行里垂直居中实测吻合）；Space Mono 13 → 19.33（58px）；Space Mono 10 → 15（45px，`DebugValueRow` 行高 = 12+15+2+18×行数+12，一行 59 实测 ✓）。
- 显式 `leading-[Npx]` 的照它取，不受影响（13/17、13/18、14/20）。
- 反解工具（临时目录，未入库）：CoreText `CTLineGetTypographicBounds` + `CTTypesetterSuggestLineBreak` 量折行，再拿截图 `probe.swift` 扫色带核对；三处独立锚点（卡片标签、行首图标列、`Toggle` 中心）都能对上 0.3pt 内。
