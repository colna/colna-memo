---
title: Biome 的 JSX 注释里不要写反引号（parse error）
date: 2026-09-17
tags: [biome, lint, jsx, react-native, pitfall]
---

# Biome 的 JSX 注释里不要写反引号

**现象**：在 JSX 子里插了一行中文注释，注释里用 `` ` `` 包了标识符（如 ``被 `settled` 归在次要 chrome``），Biome 报：

```
lint/nursery/noReactNativeRawText
parse: expected `,` but instead found `?`
```

报错位置在**注释后面那一行**的 JSX 表达式上（例如 `{cond ? <A/> : null}`），看起来完全不相关 —— 容易以为是 JSX 本身写错了。

**修法**：把 JSX 注释里的反引号去掉（用「」或什么都不加），同一句话立刻就通过：

```diff
- {/* …它本来就被 `settled` 归在次要 chrome 里 */}
+ {/* …等过渡结束再挂，和其余次要 chrome 一样 */}
```

**备注**：受影响的似乎只是 **JSX 注释**（`{/* … */}`）里的反引号；`.ts` 里的 `//` 注释写了反引号没遇到问题。中文字符本身没问题（同一条中文注释去掉反引号即可），排除「中文」这个嫌疑。

**踩到的地方**：2026-09-17 luka 会话页进场优化的 `chat/[id].tsx`（见 daily 2026-09-17）。

关联：[[rn-device-perf-measurement-via-metro-logs]]
