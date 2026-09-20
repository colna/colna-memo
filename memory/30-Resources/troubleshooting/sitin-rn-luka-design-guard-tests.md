---
title: sitin-rn 设计守门测试（shape-language / 引用式测试）
date: 2026-09-20
tags: [sitin-rn, luka, testing, design]
---

# sitin-rn 设计守门测试（apps/luka/tests/design/）

## 是什么

`apps/luka/tests/design/` 下有几个**扫描 `src/` 源码文本**的测试，不是单测业务逻辑，
而是守设计规范与文件引用。新增 UI 文件、删改被引用的组件时会撞上。

## shape-language.test.ts（扫全 src 的 .ts/.tsx）

- **未迁移屏幕**：只允许 `rounded-[3px]` / `rounded-t-[3px]` / `rounded-full`。
- **已迁移屏幕**（顶部 `MIGRATED` 名单里的文件）：圆角只允许
  8/12/14/16/18/20/22/24/28/32/36px 与 `rounded-full`；内联 `borderRadius` 同理
  （0/1/2 + 那十档）。
- **没有投影**：`shadow*` / `elevation` 一律报，白名单（EXEMPT）只有压层级的几处
  与设计单独给的例外。设计规范是「**层级靠线与留白，不靠投影**」。
- ⚠️ 两个容易踩的点：
  1. `\w*[Rr]adius:` 这个正则会命中 **`shadowRadius`** —— 就算加进 MIGRATED，投影
     还是会以「内联圆角」的名义报出来。
  2. **新 UI 文件用 Luka 新语言（20px 气泡、胶囊按钮等）时必须往 `MIGRATED` 加一条**
     （一屏改完加一条，这份名单就是迁移进度条；不要放宽全局规则）。

## 引用文件式的 design 测试

`candidate-card-fields.test.ts` 直接 `readFileSync` 具体文件（如
`components/discover-nearby/nearby-list-row.tsx`）。**删除或改名被它引用的文件时，
同一个提交里要改测试**，否则 ENOENT 挂在 CI。

## 测试红了，怎么判断是不是自己的锅

1. 看断言输出的 offenders 列表里有没有自己的文件。
2. `git diff HEAD --stat -- <offenders 里的文件>`：为空说明没碰过。
3. `git show HEAD:<文件> | rg <违规值>`：HEAD 就带违规值 → **main 上本来就红**，
   是预存失败（本仓库 2026-09-20 时 shape-language 与 candidate-card-fields 都是这种状态）。
4. 只修属于自己的那几条，别人的迁移不要顺手代劳。
