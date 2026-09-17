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
