---
title: Uniwind dev 端新加的 class 不生效（CSS 表没跟着 fast refresh 更新）
date: 2026-09-14
tags: [uniwind, tailwind, metro, fast-refresh, react-native, 静默失败]
---

# Uniwind dev 端新加的 class 不生效

## 症状

同一张卡上，**新加的** `rounded-[24px]` 没生效（渲染成直角），而同一个文件里其他类
（`flex-row` / `px-4` / `py-3`）和**旧的**任意值类（`rounded-[20px]`）全都正常。

代码没错：`git status` 干净、`pnpm lint`、`tsc`、测试全过，没人改过那个文件。

## 根因

dev 端（Metro）的 Uniwind CSS 表是 **app 启动时**加载的。合并/新增文件后 app 只做了
fast refresh，**CSS 表没有重新编译**，于是新类名在表里没有对应规则 → 静默失效。
旧类在表里，所以只有新类坏。

## 修法

整包 reload（不是 fast refresh）：模拟器上 `terminate` + `launch`，或 Dev Menu → Reload。
生产构建是整包编译，不受影响。

## 怎么认出它

**「只有新加的那个类失效、旧类正常」** 是最强特征 —— 说明不是代码、不是工具链版本，
而是运行中的 CSS 表比源码旧。

## 证据（luka，2026-09-14）

| 文件 | 时间 | 状态 |
| --- | --- | --- |
| `/tmp/hub-12.png` | 11:52 | 蜜色卡直角（合并 11:43 后只 fast refresh 过） |
| `/tmp/nohero.png` | 11:56 | 同一份代码，reload 后圆角 |

`apps/luka/src/components/edit-profile/photos-card.tsx` 的 mtime 是 11:44（合并检出），
两次截屏之间没被改过 —— 排除了「有人改了代码」。

## 证据（luka，2026-09-21：不只是任意值，核心工具类 `left-full` 同样中招）

**症状**：通知页 Activity / Visitors 分段徽标（`components/notifications/notifications-tabs.tsx`
的 `absolute -top-1 left-full ml-1`）本该贴在 label 右上，实机却压在文字左上
（用户截图 20:13，红点盖住 "Vi"）。同文件 `absolute` / `-top-1` / `ml-1` 和徽标尺寸
类都正常 —— **只有 `left-full` 失效**，正是「只有新类坏」的签名。

**排查链（dev 端：代码没错，是运行中的表旧了）**：

1. 抓 Metro 当前整包 bundle（29.8 MB）：表里有 `"left-full"` →
   `left: function(){ return "100%" }`，`min-w-[18px]` 也在。
2. 把这份表喂给 Uniwind 运行时（stub 掉 `react-native` 后直接 `UniwindStore.getStyles`）：
   `"absolute -top-1 left-full ml-1"` 解析为
   `{position:"absolute", top:-4, left:"100%", marginLeft:4}` ✓
3. Yoga 侧验证（temp 里 `yoga-layout` 复刻同构树，含 measure func）：absolute 子节点
   `left:100% + ml 4` 得到 `left = 文本宽 + 4` ✓；RN Fabric 的 `conversions.h` 也把
   `"100%"` 转成 `StyleLength::percent(1)` ✓。
4. 所以 dev app 进程里的表是旧的：它的 JS 是 fast refresh 来的（新徽标代码可见），CSS 表
   还停在徽标文件存在之前（旧类都在 → 徽标画得出来，只是没有 left）。

**2026-09-21 修正（重要）**：用户随后报告**本机打的 Ad Hoc IPA 同样错位**。IPA 是整包
构建、表在构建期生成（已从 bundle 字符串确认徽标 className / stylesheet 元数据都在），
所以「表旧」解释不了它。结论调整为：**百分比 `left`（`left-full`）不能用来做这个定位**
（dev 与 IPA 两端都落到 label 左缘，静态位），改用 `onLayout` 量 label 宽 + 点值
`style={{ left: width + 4 }}`，不依赖运行期表内容，两端一致。见下条。

**修法（已落地，2026-09-21）**：`apps/luka/src/components/notifications/notifications-tabs.tsx`
两个 label 各挂 `onLayout` 记录宽度，徽标容器改 `absolute -top-1` + inline
`left: width + 4`；宽度没量到前不渲染徽标（不会先闪在错误位置）。`apps/luka/docs/notifications.md`
《分段徽标与默认段》同步注明「不要退回 `left-full`」。

**核对手法（可复用）**：
- 从 bundle 里抠当前表：找 `Uniwind.__reinit(rt => ` 到 `, ['light'` 之间的表达式；
- 在 Node 里用 `--import` loader 把 `react-native` 映射到 stub（`Dimensions/Platform/
  Appearance/I18nManager/PixelRatio/StyleSheet`），调 `UniwindStore.reinit(gen, themes)`
  再 `getStyles(classes, {}, undefined, {scopedTheme:null, rtl:null})`；
- Yoga 面用 `pnpm add yoga-layout` 在临时目录复刻树验证百分比 inset 的落点；
- IPA 侧只想确认「代码有没有打进去」：解包 `Payload/Luka.app/main.jsbundle`（Hermes HBC），
  直接搜字符串 / `TabUnreadBadge` / className 字面量即可（表内容因 Hermes 字符串去重
  无法靠计数判断）。

## 同一类的坑

[[tailwind-arbitrary-value-whitespace]]（方括号里有空格 → 整条类名失效）：同为
「类名静默不生效」，那条是**类名本身坏了**，这条是**表旧了**。排查时先看「是新类还是旧类」。
