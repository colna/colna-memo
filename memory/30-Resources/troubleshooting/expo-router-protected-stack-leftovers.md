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

## 追加（2026-09-16，全 app 代码审查）：未声明的屏不止「残留」，还能被深链直达

同一个机制还有安全面：**不在任何 `Stack.Protected` 里的文件路由，任何时候都能被深链
打开**，与登录态 / onboarding / 删除冷静期无关。luka 里 `/dev`（Debug Tools）正是这种：
`settings/index.tsx` 只在入口手势处查 `isDevGateOpen()`，`app/dev/_layout.tsx` 自己不查，
根 Stack 也没把它放进守卫组 → `luka://dev` 生产包应可直达（待真机 `npx uri-scheme open
luka://dev --ios` 验证）。而 `stores/dev-flags.ts` 明说「flags work in production builds
too，保护就是入口门禁」——路由一旦可达，`unlimitedChat` 会直接短路 `DeductCoinMsg`
（`services/chat-billing.ts:59`），等于客户端免费 + 女方照常计收入。

修复顺序：① `dev/_layout.tsx` 渲染时 `isDevGateOpen()` 不成立就 `<Redirect>`；② 把
`dev/*` 也声明进 `Stack.Protected`（guard 用可订阅的 armed store）；③ 业务侧 devFlag 再加
`__DEV__` 双条件。同类要盘点的还有 `settings/*`、`chat/[id]`、`paywall/*`、`credits/*`
等 60 余条未声明路由——它们同样能绕开「未登录 / 未完成 onboarding / 删除冷静期」的阶段门。

教训：**`Stack.Protected` 不是全局路由守卫，只是「列进组的屏才受控」**。涉及权限/调试
开关的屏，不能只靠入口点门禁，必须屏内自查或进守卫组。

## 追加（2026-09-23）：`dismissAll()` 清不掉根栈里的二级页

`popToSessionRoot()` 原实现是 `router.dismissAll()`，以为「登出前 `(tabs)` 还在，会退回它」。
实际上 `dismissAll()` 派发的 `POP_TO_TOP` **没有 target**，会由当前**最内层的 stack** 处理：

- 人站在 `/settings/logout`（`settings` 是个嵌套 Stack）时，`dismissAll()` 只把嵌套栈退到
  `settings/index`；根栈仍是 `[(tabs), settings]`，`settings` 还押在顶上。
- 随后 `logout()` 翻守卫：`settings` 被移出 routeNames、在**没有转场**的情况下从原生栈抽走，
  留下残影 —— 2026-09-23 测试反馈：登出后不重启、重新登录走 onboarding，**每一步都闪一下
  Settings**；重启（原生栈重建）后消失。
- 旧报告「重新登录落在 Settings」在 `1c8942781`（文件路由穷举声明）之后不复现，所以这个
  函数一直看起来「有效」，实则根栈从来没被它清过。

修法：用带 target 的 `POP_TO` —— `router.dismissTo("/(tabs)")`。`getNavigateAction` 会按
当前 state 与目标 state 的分歧点算出 target（这里是根栈），根栈的 StackRouter 才收到
`POP_TO`，把 `settings` 真退掉。

判据（可复用）：**想知道一次「清栈」有没有清到根，别只看 `canDismiss()` / `dismissAll()`
是否被调，要看派发的 action 有没有 target、target 是不是根 navigator。没有 target 的
stack action 一定先给最内层。**

## 相关

- [expo-router：深链冷启动进二级页，返回键报 GO_BACK](expo-router-deeplink-back-empty-stack.md)
