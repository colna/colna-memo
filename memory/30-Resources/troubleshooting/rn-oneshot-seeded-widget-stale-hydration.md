# 一次性落座的组件 + 多来源 hydration：用陈旧值把真值覆盖掉

**症状**：同一个屏，两次挂载显示**两个不同的值**（本次实测 28 与 63）。页面不报错、不发警告；
最坏的结果不是「显示旧值」，而是用户点一下 Save，就把服务端的新值用旧值盖掉了 —— 静默的数据损坏。

**典型场景**：一个「只在挂载那一刻读初始值」的控件（滑尺、picker、地图视图、原生控件 wrapper），
配上一条会**先给本地缓存/登录快照、后给服务端真值**的 hydration 链。

```ts
// apps/luka/src/hooks/use-me.ts —— 服务端 overview 没回来之前，先用登录快照顶上
const snapshotMe = snapshotName ? { displayName, age: snapshot?.age ?? 0, ... } : null;
const me = meWithSnapshotLocation(data?.me ?? null, snapshotMe); // data 为空时 = 快照
```

```tsx
// ❌ 第一版：`me` 一非空就落座
useEffect(() => {
  if (seeded.current || !me) return;
  seeded.current = true;
  setAge(me.age > 0 ? me.age : FALLBACK_AGE);  // 写进快照/兜底值，之后永不再改
}, [me]);
```

`AgeRuler` 的 `initialIndex` 是个 **ref**，只认挂载那一刻的年龄 —— 「先占位、后更新」在这类组件上
根本不存在，落座即锁死。

**为什么容易漏**：两个来源都是「合法数据」，类型一致、无 null 警报；而且**快照与真值经常恰好相等**，
本地复现不出来。本次能发现纯属巧合（测试账号的注册年龄 28 与后来改到的 63 不同）。

**修法**：加一道「等首屏请求落地」的闸，读**同一个**页面显示用的那个源。

```tsx
const { me, initialLoading } = useMe();        // initialLoading：无缓存的首屏请求在飞
const storedAge = me?.age ?? 0;                 // 与 hub 那一行同源
useEffect(() => {
  if (seeded.current || initialLoading || !me) return;
  seeded.current = true;
  setAge(storedAge > 0 ? storedAge : FALLBACK_AGE);
}, [initialLoading, me, storedAge]);
```

前提是 store 把「数据」与「loading: false」放在**同一次 setState**，否则会出现
`loading=false && 数据还没到` 的空窗，闸门形同虚设 —— 落座前先读一眼 store 的写入顺序
（`apps/luka/src/stores/me-overview.ts` 是这么写的：`{ data, loading: false, error: false }` 一次写入）。

**怎么验证「落座读的是哪个源」**（纯读代码分不清，两个值都可能对）：

1. 把兜底常量临时改成**哨兵值**（`FALLBACK_AGE = 99`、不可能出现在真实数据里的数字）；
2. **冷启动**（`xcrun simctl terminate booted <bundleId>` → `luka://expo-development-client/?url=…`），
   这样才会走到 hydrate 链的第一段；
3. 打开目标屏截图：显示真值（63）→ 闸门生效；显示哨兵（99）或快照值（28）→ 落座早了；
4. 改回真值，再跑一遍确认。

**推广**：任何「只在挂载时读一次初值」的控件都要问两个问题 ——「这个初值来自本地缓存还是服务端？」
「缓存可能比服务端旧多少？」答不上来就不要直接落座。

相关：[`verification-discipline.md`](verification-discipline.md)、
[`rn-simulator-pixel-measurement.md`](rn-simulator-pixel-measurement.md)。
