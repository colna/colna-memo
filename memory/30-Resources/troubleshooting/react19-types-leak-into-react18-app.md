---
title: React 19 类型污染 React 18 子应用
date: 2026-11-17
tags: troubleshooting, react, typescript, pnpm, monorepo
---

# React 19 类型污染 React 18 子应用

## 现象

pnpm monorepo 里同时有 React 18（admin）和 React 19（web）两个 app 时，admin 的 `tsc --noEmit` 对几乎所有 antd/Umi 组件报：

```
TS2786: 'ConfigProvider' cannot be used as a JSX component.
Type 'ReactNode | Promise<ReactNode>' is not assignable to type 'ReactNode'.
```

错误里引用的 `@types/react` 路径是 `@types+react@19.3.0`，但 admin 自己 `node_modules/@types/react` 明明是 18.x。

## 根因

pnpm 把 19 版本 hoist 到 `node_modules/.pnpm/node_modules/@types/react`。antd/rc-util 等依赖的 `.d.ts` 从 `.pnpm/<pkg>/node_modules` 向上查找 `@types/react`，先命中这个 hoist 的 19 版本；admin 源码用 18 版本，两个 `ReactNode` 不兼容。

## 修法

在 React 18 子应用的 `tsconfig.json` 里强制类型解析到自己安装的 18.x：

```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "react": ["node_modules/@types/react"],
      "react/*": ["node_modules/@types/react/*"],
      "react-dom": ["node_modules/@types/react-dom"],
      "react-dom/*": ["node_modules/@types/react-dom/*"]
    }
  }
}
```

改完重新跑 `tsc --noEmit` 即可。不要用全局 override 把 19 降到 18，会反过来破坏 React 19 的 app。
