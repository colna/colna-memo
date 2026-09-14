---
title: Uniwind dev 端新加的任意值 class 不生效（CSS 表没跟着 fast refresh 更新）
date: 2026-09-14
tags: [uniwind, tailwind, metro, fast-refresh, react-native, 静默失败]
---

# Uniwind dev 端新加的任意值 class 不生效

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

## 同一类的坑

[[tailwind-arbitrary-value-whitespace]]（方括号里有空格 → 整条类名失效）：同为
「类名静默不生效」，那条是**类名本身坏了**，这条是**表旧了**。排查时先看「是新类还是旧类」。
