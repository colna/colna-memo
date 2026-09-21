---
title: expo-router：叶子路由与同名目录能共存；不起 Metro 的静态验证法
date: 2026-09-21
tags: [troubleshooting, expo-router, react-native, sitin-rn, routing]
---

# expo-router：叶子路由与同名目录能共存

出处：sitin-rn `apps/luka` T0056 —— 在已有 `app/profile/[id].tsx` 旁边新增
`app/profile/[id]/posts.tsx`，需要确认 `[id]` 同名的「文件 + 目录」不冲突。

## 结论

**可以共存。** `getDirectoryTree` 把文件放在 `directory.files`、子目录放在
`directory.subdirectories` 两张表里，只有**同一个 route 名 + 同一 specificity** 的
文件才会抛 `conflict on the route`。所以：

- `/profile/123` → `profile/[id].tsx`
- `/profile/123/posts` → `profile/[id]/posts.tsx`

不用为了绕开「假想的冲突」把路由摊平成 `user-posts.tsx?id=`。

## 不起 Metro 的静态验证法

新建路由后不必等模拟器：用 expo-router 自带的 `getRoutes` 直接跑一遍路由树。

```js
process.env.NODE_ENV = "production"; // 否则 dev 会 loadRoute 每个文件、TSX 直接 SyntaxError
const requireContext = require(".../expo-router/build/testing-library/require-context-ponyfill.js").default;
const { getRoutes } = require(".../expo-router/build/getRoutes.js");

const real = requireContext(appDir, true, /.*/, {}); // 第 4 参是 files 对象，别传 true
const ctx = () => ({ default: () => null });          // stub，防止真的 require TSX
ctx.keys = () => real.keys();
ctx.resolve = (k) => k;
ctx.id = "0";
const routes = getRoutes(ctx, { skipGenerated: true, ignoreEntryPoints: true, importMode: "lazy" });
```

递归 `children` 打印 `route` 即可看到全部路径；有冲突会在 dev 下抛错、静态跑则看是否
出现在列表里。脚本与运行记录在 `/var/folders/.../opencode/luka-bugs/routes-check.js`。

## 坑

- ponyfill 的 `requireContext(dir, recursive, regexp, files)` 第 4 参是**累积对象**，
  传 `true` 会报 `Cannot create property ... on boolean`。
- 直接跑 `getRoutes`（NODE_ENV=development）会走进 `validateRouteTreeExports`，它逐个
  `loadRoute()`；Node 不认识 TSX，报的是 `Unexpected token '<'`，与路由本身无关。
- 另一个更重的验证：Metro 已在跑时直接 `curl ".../<file>.bundle?platform=ios"`，能编译
  过就说明该模块进了图（本次 200 + bundle 里出现新页面字符串）。
