---
title: Tailwind 任意值里有空格 → 整条类名静默失效
date: 2026-09-08
tags: [tailwind, uniwind, react-native, 批量替换, 静默失败]
---

# Tailwind 任意值里有空格 → 整条类名静默失效

## 症状

全 App 的弹层遮罩同时变透明：确认框、礼物面板、Boost 面板、举报、图片查看器、
Android 文本编辑弹层……背后都不再压暗。

**没有任何报错。** lint 过、typecheck 过、测试全绿、Metro 不吭声。

## 根因

Tailwind 的**任意值（方括号）里不能有空白**。有空格就把类名截断，整条声明失效：

```tsx
className="bg-[rgba(74, 53, 39, 0.4)]"   // ❌ 那一层是透明的
className="bg-[rgba(74,53,39,0.4)]"      // ✅
className="bg-[rgba(74,_53,_39,_0.4)]"   // ✅ 下划线会被还原成空格
```

## 怎么踩进去的

按颜色做**全局批量替换**时踩的，而且很容易再踩一次：

- 源里的取值是**紧凑**写法：`rgba(17,24,38,`
- 替换串是从 `theme.js` 里**格式化过的 CSS** 抄来的：`rgba(74, 53, 39, `

两边一个紧凑一个带空格，替换完 12 个文件同时失效。批量换色的人不会去逐个打开弹层看。

## 修法

```bash
# 去掉方括号内的所有空白
python3 - <<'PY'
import re, pathlib
for p in pathlib.Path("src").rglob("*.tsx"):
    t = p.read_text()
    new = re.sub(r"\[(rgba\([^\]]*\))\]", lambda m: "[" + re.sub(r"\s+", "", m.group(1)) + "]", t)
    if new != t: p.write_text(new)
PY
```

## 守住它

加一条测试，扫 `-[…]` 方括号里有没有空白：

```ts
const ARBITRARY = /-\[([^\]]*)\]/g;
for (const m of line.matchAll(ARBITRARY)) if (/\s/.test(m[1])) hits.push(...);
```

真源：`sitin-rn/apps/poo/tests/design/arbitrary-values.test.ts`。

## 同一类的坑

**批量换色扫不到的地方**，全都要单独查一遍 —— 它们不在 `src/` 下：

| 位置 | 只在什么时候看得见 |
| --- | --- |
| `app.config.ts` 的 splash 底色 | 冷启动第一帧 |
| `app.config.ts` 的 `adaptiveIcon.backgroundColor` | 桌面图标 |
| `src/assets/*.svg` 里的 `fill` | 桌面图标 |
| 烤进视频/图片里的底色 | 那个容器里 |
| 注入给原生模块的 theme 对象（如 `CallRechargePresenterTheme`） | 那张原生面板出现时 |

最后一项还叠了个格式坑：那个包收的是 **`#AARRGGBB`（alpha 在前）**，而 `#E8F3FF00`
按 `#RRGGBBAA` 写就成了 91% 不透明的亮黄。**8 位色一定要确认字节序**。
