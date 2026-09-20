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

## 补充（2026-09-20，All chats 搜索框 —— 单行 TextInput 同样吃行盒）

第三条规律，从 `apps/luka/src/components/chats-play/all-chats-drawer.tsx` 的搜索框反解出来：

- **症状**：抽屉里 48pt 高的搜索行内，`TextInput`（`flex-1 font-sans text-[16px]`，无 height / 无 `leading-`）的 placeholder 只露上半截；iOS 光标却是完整行高（22pt ≈ Nunito 16 的自然行高），光标底 ≈ 字形被裁的那条线。
- **逐像素反解**（820×1706 的缩放截图，1.79 px/pt）：输入框内高 83px ≈ 46pt（= 48 − 双 1px 描边）；光标 578..617 = 40px ≈ 22pt；字形 cap 顶在 608、基线约 628，即**基线落在框底之下约 6pt**，剩下的部分被框裁掉。所以问题不是字号小，而是基线相对盒子下移。
- **判据**：luka 里**单行 TextInput 必须自己钉行盒** —— `height`（`sign-in-field` 的 `height: 24, padding: 0`、`edit-profile/*` 的 `FIELD_HEIGHT`、dev 屏的 `h-[46px]`）或 `leading-[Npx]`（`delete-confirm-field` 16/22、`chat-input-bar` height + lineHeight）二者至少其一。全 `apps/luka` 扫一遍，没钉的就这一处抽屉搜索框（另有 `leave-link-dialog-host`、`location-picker` 两处待观察，都是「无 height 无 leading」）。
- **修法**：`className="... h-full ... leading-[21px]"`；`h-full` 取父行内容高，行高将来变不用再改。`Text` 上 Tailwind 的 `text-[Npx]` 不给 lineHeight（见上一条 twMerge 的坑），`TextInput` 同理。
- **注意**：`pnpm --filter luka typecheck` / `biome check` 都拦不住这类问题，只能肉眼在模拟器里看；模拟器复核前别当已验证。

## 补充（2026-09-20，sign-in 邮箱框 —— 原生 placeholder 被 iOS 自己挪位）

同日第二处「文案被裁」，在 `sign-in` 页 EMAIL 框（用户反馈「文案展示不全」）。**机制与上面那条不同**：这一处输入框的行盒是钉住的（`height: 24, padding: 0`），裁掉字头的是 **iOS 自己摆的那个 placeholder label**。

- **症状**：EMAIL 框的 `Enter your email` 只露下半截，密码框完好 —— 它是没被点过的那个。
- **根因**（`apps/blueprint/src/components/auth/field-placeholder.tsx` 早已记录）：iOS 在字段成为 first responder 的那一刻会重摆一次 placeholder label；这一屏键盘弹起时整张表单跟着 `KeyboardAvoidingView` 位移，把那个瞬间的 frame 递给了 UIKit，label 留在错位处、被输入框裁掉一半。光标和真正的文字从 line fragment 画，不受影响 —— 所以这坑难按需复现，**只有先点的那只框中招**。
- **修法**：照蓝图的解法 —— `apps/luka/src/components/login/sign-in-field-placeholder.tsx` 在 JS 层画占位（`absolute inset-0 justify-center px-4` + `font-sans text-[16px] leading-[20px]` + `pointerEvents="none"` + 对读屏隐藏），输入框不再传原生 `placeholder`（改 `leading-[20px]` 钉行盒、高度给满整行 56、`textAlignVertical: "center"` 兜 Android），`accessibilityLabel` 补住读屏；`{placeholder && !value}` 控制显隐。
- **验证**：iPhone X / iOS 26.5 模拟器，焦点停在 EMAIL 框（光标可见）时占位文案完整、与光标同一行；截图标尺复核 `leading-[20px]` 不裁 Nunito 16 的降部。回归测试 `apps/luka/tests/design/guest-path.test.ts` 钉住「不走原生 placeholder」。
- **判据**：凡是「原生 placeholder + 键盘弹起会位移表单」的屏，都该换成 JS 画；蓝图 `FieldPlaceholder` 是现成参照。
