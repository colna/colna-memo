---
title: expo-router：Stack.Protected 只管它声明过的屏，登出后残屏会留在栈里
date: 2026-09-10
tags: [troubleshooting, expo-router, react-native, sitin-rn, navigation]
---

# expo-router：Stack.Protected 只管它声明过的屏

出处：sitin-rn `apps/luka`，用户报「Debug 页删完账号 → 登录页 → 点 Continue → 落在
**Settings**，而不是新账号该走的 onboarding」（PR #484）。

## 症状

登出（或删号）之后重新登录，**落在上一次退出前停留的那个二级页**，而不是该去的首页 /
onboarding。看起来像路由写错了，实际上路由是对的。

## 根因

`app/_layout` 用声明式守卫分组：

```tsx
<Stack.Protected guard={authed && profileComplete}><Stack.Screen name="(tabs)" /></Stack.Protected>
<Stack.Protected guard={authed && !profileComplete}><Stack.Screen name="onboarding" /></Stack.Protected>
<Stack.Protected guard={!authed}><Stack.Screen name="splash" /><Stack.Screen name="login" /></Stack.Protected>
```

**守卫只移除它自己显式声明的那些屏。** `settings/*`、`dev/*` 这类按文件自动注册的路由
不在任何一组里，登出时不会被移走：

```
Settings → Debug → 删除 → logout() → replace("/login")
栈：[settings, login]        ← (tabs) 被守卫移走了，settings 没有
重新登录 → login 被守卫移走 → 栈顶又变回 settings
```

## 修法与**时机**

清栈必须发生在**登出之前**：

```ts
export function popToSessionRoot(): void {
  if (router.canDismiss()) router.dismissAll();   // 只剩一屏时 dismissAll 会抛
}
```

- 那一刻 `(tabs)` 还在，`dismissAll()` 退回到它；
- **登出之后再清就晚了** —— 栈里第一个还活着的屏正是 `settings` 自己，等于没清。

守卫是声明式的、render 时就生效，所以任何「先改状态再清栈」的写法都晚一步。四条登出
路径（Debug 删除 / Debug 重置身份 / Settings 登出 / Settings 删号）都要调。

## 根治

把所有「登录后才该存在」的屏也声明进守卫组。代价是要逐条列出二十多个路由，所以先用
上面那个函数兜着。

## 连带教训

这类 bug 常常和另一个 bug 叠在同一个用户操作上（当时是「删号请求本身也不成功」）。
**只修一个都看不出效果** —— onboarding 只有在删除真的成功、拿回 pending 新账号时才会
出现。排查时先把两条因果链拆开，否则会误判第一处没修好。
