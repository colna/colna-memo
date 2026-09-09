---
title: sitin-rn 派生包整套换视觉的做法(token 同位替换 + 形状语言白名单)
date: 2026-09-09
tags: [sitin-rn, react-native, design-token, figma, 排错]
---

# 派生包换视觉:换值不换键 + 按目录守两套形状规则

`apps/*` 从 `apps/blueprint` 派生出来时带着蓝图那套视觉(白底 / 蓝前景 `#1D4ED8` / 圆角只有 3px / Inter)。按自己的设计稿落地时,坑不在写屏幕,在**怎么换掉那一整套 token 而不炸掉其余几十屏**。

## 1. 颜色:同位替换,键名一个都不改

`design/theme.js` 的键被 ~298 处 `Colors.*` 和 ~1197 个 className 引用。**只换取值、不换键名**,全 app 自动跟着变;换键名要改一千多处,且没有任何守卫。

```bash
sed -i '' -e 's/#1D4ED8/#684431/g' -e 's/#111826/#3A2317/g' ... design/theme.js
pnpm <app>:tokens          # 重生成 src/global.css + src/theme/colors.ts
pnpm --filter <app> tokens:check
```

⚠️ `#FFFFFF` **不能全局替换** —— 它同时是「页面底」和「按钮上的字」。按行号分别处理:面→奶油,`*-foreground`→保持白。

键名里残留的颜色词(`paywall-blue`、`coralTint`、`chatPrimary`)**按语义读,别按名字读**;顺手把 `theme.js` 顶部那段文档改掉,否则下一个人会照着「蓝色是前景」去理解一套棕色。

## 2. 形状:白名单渐进迁移,不放宽全局规则

`tests/design/shape-language.test.ts` 强制「圆角只有 3px 和整圆」。新设计要胶囊 + 16px 卡片时,**不要改这条规则的取值**(全库几百处 3px 会一起挂),改成**按目录守两套**:

```ts
const MIGRATED = [/^app\/login\.tsx$/, /^app\/onboarding\//, /^components\/onboarding\//, ...];
// 未迁移目录:只许 rounded-[3px] / rounded-full
// 已迁移目录:只许 rounded-[16px] / rounded-full
```

**这个白名单就是迁移进度条** —— 一屏改完挪一个进去。好处是「哪些屏已经是新样子」有机械答案,不用翻 git log。

## 3. 三个会咬人的地方

| 现象 | 根因 | 修法 |
|---|---|---|
| `pnpm boundaries` 报 `identity leak: contains "blueprint"` | 它扫的是**文本**,不是 import —— 中文注释里提一句「blueprint 那套按钮」就算泄漏 | 注释里改说「模板」/「派生来源」 |
| `pet-flat-background.test.ts` 挂 | `src/assets/pet/pet-flat.mp4` 无 alpha,底色被**烤进视频**,与 `Colors.tagBg` 绑死 | `pnpm render-pet-flat <app>`(要 ffmpeg) |
| `design/README.md` 写着别的包的名字和命令 | `pnpm new-app` 派生时没重写这个文件 | 落地新视觉时顺手重写,别信里面的色表 |

## 4. 还有两处烤死了主题色,换色时一起查

- `src/assets/pet/pet-flat.mp4` → `Colors.tagBg`(有测试守)
- `app.config.ts` 的 splash / `adaptiveIcon.backgroundColor` → `src/assets/icon.svg` 自己的底色(**没有守卫**)。只换主题不换图标,会得到「启动蓝底、进 app 奶油底」的跳变;要一起改得 `pnpm render-icon <app>` 重出图标。

## 5. 分支命名

`sitin-rn` 的 `AGENTS.md` 规则 3:**⛔ 不许 `feature/<App名>` 这类常驻集成分支**,只许 `personal/<人>/<事>` 或 `feat|fix|docs|chore/<事>`,且合入目标唯一是 `main`。建分支前先看 AGENTS.md。

**相关**:[sitin-rn / sitin-rn2 双 clone 跑错目录](sitin-rn-multi-clone-wrong-app.md)
