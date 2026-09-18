---
title: expo-router：useSegments() 在 index 路由上不含 "index" 段
date: 2026-09-18
tags: [troubleshooting, expo-router, react-native, navigation, sitin-rn]
---

# expo-router：`useSegments()` 在 index 路由上不含 `"index"` 段

**症状**（2026-09-18，Luka 底栏按 tab 切换材质）：在 Chats 页（`(tabs)/index.tsx`）判
`activeRouteName === "index"` 恒为 false，条件分支永远走不到；同一个表达式在
scene / discover 等 tab 上正常。

**根因**：`useSegments()` 返回的是**占位的路径段**。index 是默认子路由、URL 就是 `/`，
它**不占** segment ——

- Chats 页：`["(tabs)"]`（长度 1，`segments[1]` 是 `undefined`）
- Scene 页：`["(tabs)", "scene"]`

代码里那行注释「typed routes 下 segments 是定长元组的联合，有些成员没有 index 1」
说的就是这件事，但只有真正依赖 `segments[1]` 取值时才会暴露。

**修法**：缺段时回落到导航器 state 的 focused route（它永远有名字）：

```ts
const activeRouteName = onTabsRoot && segments[1] ? segments[1] : state.routes[state.index]?.name;
```

**怎么快速定位**：临时在 UI 上渲染 `{String(activeRouteName)} | {JSON.stringify(segments)}`，
`simctl` 截图直接读出 `undefined | ["(tabs)"]` —— 比翻源码 / 猜快得多（改完记得删）。

**同族注意**：任何「用 segments 拼路由名」的写法（埋点、按路由切样式、判断当前 tab）
都要考虑 index 缺段与 pushed route 两种情况；判断「是否在 tabs 根」用
`segments[0] === "(tabs)"` 是安全的（组长存在），取 tab 名不是。

## 关联

- [[expo-router-protected-stack-leftovers]]
- [[ios-simulator-keyboard-and-taps]]（§9 截图取像素/调试 UI 验证）
