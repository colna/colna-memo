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

## 追加二（2026-09-23）：残影跟着「登出瞬间的顶屏」走 —— 根治要换 navigator key

上面那条 `dismissTo` 修法把顶屏从 `settings` 换成 `(tabs)` 之后，测试同学录屏里的残影
也跟着变成了 **Chats**：登出后不重启，之后**每一次**导航转场都先闪一下 Chats（加载骨架 +
上一个账号的红点底栏），不是只在 onboarding 闪。也就是说残影不是某一屏的 bug，而是
**被移走的顶屏留在原生 `RNSScreenStack` 里当后续转场的基图**：guard 换组只增删 JS 路由，
原生栈容器不重建。

根治：根 `Stack` 按会话阶段加 key（`key={authed ? "authed" : "guest"}`），阶段一换
整棵原生栈重建，旧视图连同残留一起释放；`onboarding → tabs` 同属 authed，不重建。
配套不变量测试：`tests/design/protected-routes.test.ts`。commit `c4c07d87d`。

判据补充：**只清栈不够，阶段边界要换 navigator key**。清栈解决「哪一屏留在 JS 栈里」，
换 key 解决「原生容器把哪一屏当转场基图」。

定位手法：录屏抽帧（`ffmpeg -vf "fps=10,scale=...tile=6x8"` 出接触表，再按时间点取
全分辨率帧）能直接看出「闪的是真实路由还是残留」—— 残留帧里带的是上一个账号的徽标数据。

⚠️ **真机复验证伪**：测试同学的包确认包含 `c4c07d87d` 后仍会闪 —— 换 key 不足以释放
原生层留下的那份视图。终解见下。

## 追加三（2026-09-23 · 终解）：别让 guard 在同一帧拆嵌套容器，退出前先换成空白盖

残影的准确机制：**嵌套 navigator 容器**（`(tabs)`、`settings` 这类带 `_layout` 的屏）
在 guard 换组的**同一帧**里既被移除、父容器又换了内容时，原生 screen container 会把它的
视图留下当作后续转场的基图（同类问题见 rns #4504：「nested ScreenContainer 在同一帧被
移除会留下孤儿 fragment」）。这也解释了为什么「顶屏是谁，残影就是谁」：
Settings → `(tabs)` → Chats 一路跟着走。

终解（commit `67164e16e`）：退出路径不再让 guard 直接拆嵌套容器 ——
`prepareSessionExit()` 先用 `dismissTo("/(tabs)")` 退掉二级页，再用**普通导航**把 `(tabs)`
自己换成 `/session-cover`（一屏整屏底色的过渡屏，`guard={true}`、声明在最后不抢
`routeNames[0]`）；`logout()` 翻守卫时栈里只剩一屏普通屏，没有嵌套容器可漏，随后 replace
到登录页（删号路径是 `/account-deletion`）。

验证手法（模拟器可验 JS、验不了原生）：临时 `__repro` 脚本 + 模块级 interval 打根栈
state，实测 `prepareSessionExit()` 后收敛为 `*session-cover`、`logout()` 后为 `*login`；
原生残影模拟器复现不了，仍需真机复验。

可复用结论：**会话边界不要在同一帧里拆带 `_layout` 的嵌套屏**。要么先退到普通屏再翻状态，
要么让那个容器先离开导航栈。

## 相关

- [expo-router：深链冷启动进二级页，返回键报 GO_BACK](expo-router-deeplink-back-empty-stack.md)
