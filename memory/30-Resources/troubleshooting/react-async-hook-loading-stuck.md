---
title: 取数 hook 的骨架屏卡死：被取消的请求连 loading 也不收尾
date: 2026-09-10
tags: [troubleshooting, react, react-native, hooks, sitin-rn]
---

# 取数 hook 的骨架屏卡死

出处：sitin-rn `apps/luka` 的 `useFriends`，真机上 Friends 页一直停在骨架屏
（PR #483）。这是所有「`useCallback` + `cancelled` 闭包」写法共有的坑。

## 症状

列表永远是骨架屏，不报错、不超时。看起来像接口不返回。

## 两处叠在一起

### 1. 被取消的请求跳过了 `setLoading(false)`

常见写法：

```ts
const reload = useCallback(() => {
  let cancelled = false;
  setLoading(true);
  fetchIt()
    .then(r => { if (!cancelled) setData(r); })
    .finally(() => { if (!cancelled) setLoading(false); });   // ⛔
  return () => { cancelled = true; };
}, []);
```

**只要 effect 重跑过一次、而新请求没能收尾，`loading` 就永远是 true。** `cancelled`
本意是挡住过期数据，却顺手把「关掉 loading」也挡掉了。

**修法：换成自增请求号 —— 过期请求只是不写数据，当前那一次一定收尾。**

```ts
const requestId = useRef(0);
const reload = useCallback(() => {
  const id = ++requestId.current;
  setLoading(true);
  fetchIt()
    .then(r => { if (id === requestId.current) setData(r); })
    .finally(() => { if (id === requestId.current) setLoading(false); });
}, []);
```

代价是卸载后可能多一次无害的 `setState`（React 18 起不再警告），换来这一类卡死不可能
再发生。

### 2. 把「显示什么」的开关塞进了取数的依赖数组

我给这个 hook 加了一个 Debug 开关（显示假数据），顺手写进了 `useCallback` 的依赖里。
于是**开关一变（包括它在启动时那次异步 hydrate）整个请求被重新武装**：旧请求被
`cancelled` 掐掉 → 撞上第 1 条 → 卡死。

**取数的生命周期不该由一个「显示什么」的开关决定。** 请求的依赖数组保持原样，开关只在
**返回值**那一步顶替结果：

```ts
return useMock ? { data: MOCK, loading: false } : { data, loading };
```

假数据数组要放在模块级算一次（身份稳定），否则每次渲染造一个新数组，把下游那串
`useMemo` 全部作废。

## 排查提示

- 「一直在骨架屏」先看 `loading` 会不会被关，而不是先怀疑接口。
- Fast Refresh 会留下旧的 hook 闭包，改完这类代码要**完整重启 App**，不能只 reload。
