---
title: 派生包的语义色「成对」假设会静默失效（toast 白对白）
date: 2026-09-15
tags: [uniwind, theming, blueprint, 派生包, react-native, 静默失败]
---

# 派生包的语义色「成对」假设会静默失效

## 症状

Luka 的全局 toast 弹出来是一块**近乎空白的白圆角矩形**：文字其实在（`Sent to 2100047057`），
但和底色几乎同色，看上去像没渲染。

`git status` 干净、lint / typecheck / 测试全过 —— 没人改过那个文件。

## 根因

模板 blueprint 的 toast 写的是：

```tsx
style={{ borderColor: Colors.primary, backgroundColor: Colors.panel }}   // panel = #FFFFFF
<Text className="... text-background">                                   // background = #FFFFFF
```

blueprint 里 `panel` 与 `background` **同为白**（本来就已白对白）；Luka 派生后主题整套换成
暖色，`background` 变成奶油 `#FDF9EE`，`panel` 仍是白 —— 于是从「白对白」变成「白对近白」，
两边都读不出来。

教训不是「不该用 `text-background`」，而是：**`bg-A` + `text-B` 的对比是跨 token 的假设，
派生包把整张色表同位替换后，这个假设不会自动被检查**。这类 bug 不报错、不破坏测试，
只有在真机/模拟器上看才会发现。

## 识别与修法

- 判据：浅色浮层上的字几乎看不见；`rg "text-background|text-foreground"` 找成对写法，
  逐个核对**当层底色**（不是页面底色）与文字色是否真成对。
- 修法：浮层自己决定对比 pair（Luka 重做后是 `bg-foreground` + `text-background`，两个 token
  在同一个元素上互为底/字），不要用 `Colors.panel` 这类「与页面底同色」的面去接别处的字色。
- 模板来源也可以顺手看一眼：blueprint 的同款组件同样白对白（它自己的 `background` 也是
  `#FFFFFF`），派生前就存在的静默问题。

## 附：沉睡的接缝同样不会报错

同一个文件旁边，Luka 的 `stores/bottom-bar.ts` 注释写着「Floating UI (the toast) reads this
rather than guessing from the route」，但全仓 `useBottomBarStore` **只写不读**（tab bar /
chat composer 都 report 了高度，没有消费方）—— 从 iris 抄过来的避让机制一直睡着。
重做 toast 时接上它即可（底部浮层自动避开标签栏 / 输入栏），不需要新机制。

**判据**：接缝（store / hook / config 注入点）存在 ≠ 被使用；抄完一个包后 grep 一次
「谁在读它」，没有读方的接缝就是下一轮重做时要接上的东西，而不是新造的轮子。
