---
title: Instagram 脚本选择器约定
date: 2026-06-25
tags: instagram, dom, selector, ins-script, automation, frontend
---

# Instagram 脚本选择器约定

来源:`sitin-next/packages/app-ins-scripts`(及 `social-proxy-scripts-container-app/scripts/instagram`)实际代码分析。

## 核心约定:禁用 className 选择器

操作 Instagram 页面 DOM 时,**不要用 class 选择器**:

- ❌ `getElementsByClassName('...')`
- ❌ `querySelector('.someClass')` / `querySelectorAll('.someClass')`
- ❌ `[class*="..."]` 等 class 属性选择器

验证:app-ins-scripts 全量代码中以上三类用法均为 **0 处**(152 处 DOM 选择全部走属性/标签选择器)。

## 为什么

Instagram 的 class 名是**构建时混淆/随机生成**的(每次发版都会变),用 class 选择器极易在版本更新后失效。因此脚本刻意只依赖**稳定锚点**。

## 推荐用法(稳定锚点,按优先级)

| 锚点 | 示例 |
|------|------|
| ARIA role | `querySelector('[role="button"]')`、`querySelectorAll('div[role="dialog"]')` |
| aria-label(支持多语言匹配) | `querySelector('svg[aria-label="..."]')`、`svg[aria-label*="Waveform"], svg[aria-label*="波形"]` |
| href(路由路径) | `querySelector('a[href="/direct/inbox/"]')` |
| data-* 属性 | `querySelectorAll('[data-visualcompletion="ignore"]')` |
| 语义属性 | `span[title]`、`[role="slider"]`、`[aria-labelledby*="mid."]` |
| 标签 / 结构 | `querySelectorAll('span, div')`、`button`、`h2`、`div[style]` |

## 实践要点

- 多语言场景用 `aria-label*=` 子串匹配,并把中英文 label 并列(`'svg[aria-label*="Waveform"], svg[aria-label*="波形"]'`)。
- 选择器集中维护成常量数组(如 `AVATAR_SELECTORS`、`DISMISS_SELECTORS`、`BAN_ARIA_SELECTOR`),用 `.join(',')` 一次匹配,便于改版时统一调整。
- `className` 字样若出现在代码里,是读取元素的 class **属性**做文本判断,不作选择器用。
