---
title: 缺原生模块时 expo-* 的 import 期抛错：try/catch 拦不住（Expo dev Metro 的 guardedLoadModule）
date: 2026-09-17
tags: troubleshooting, expo, react-native, metro, native-module
---

# 缺原生模块：`try { require("expo-xxx") } catch` 是假兜底

**症状**（2026-09-17，Luka 真机 dev build，基包还是加依赖之前那一版）：

```
Uncaught Error: Cannot find native module 'ExponentPedometer'
Source: src/hooks/use-shake.ts (58:21)
  requireNativeModule (expo-modules-core/src/requireNativeModule.ts:20)
  <global> (expo-sensors/build/ExponentPedometer.js:2)
  <global> (expo-sensors/build/Pedometer.js:4)
  <global> (expo-sensors/build/index.js:1)      ← barrel 第一行就 import ./Pedometer
  loadAccelerometer (use-shake.ts:58)           ← 这里明明是 try/catch 包着的
```

## 根因：Expo dev Metro 的 require 会「先报红屏，再把异常吞掉」

`node_modules/@expo/cli/build/metro-require/require.js` 里：

```js
let inGuard = false;
function guardedLoadModule(moduleId, module, moduleIdHint) {
  if (!inGuard && global.ErrorUtils) {
    inGuard = true;
    let returnValue;
    try {
      returnValue = loadModuleImplementation(moduleId, module, moduleIdHint);
    } catch (e) {
      global.ErrorUtils.reportFatalError(e);   // ← 红屏就是这里出的
    }
    inGuard = false;
    return returnValue;                        // ← 异常被吞了（返回 undefined）
  }
  ...
}
```

也就是说：**首次**初始化某个模块时抛出的异常会被直接 `reportFatalError`（红屏）并吞掉，
调用方的 `try/catch` 只能拦住「第二次 require」——红屏已经弹出来了。

两条推论：

1. **`try { require("某原生包") } catch {}` 不能用来做「缺原生模块时降级」**——那个包只要在
   import 期调 `requireNativeModule`，红屏就躲不掉。要么保证基包里有它，要么**先探测**。
2. 同一个机制还会把**误导性的红屏**端上来：`loadModuleImplementation` 失败后
   `module.publicModule.exports = undefined`、`hasError = true`，后续依赖它的模块可能抛出
   完全不相干的 `ReferenceError: Property 'xxx' doesn't exist`（Hermes 对裸标识符的措辞），
   把排查带偏。**判断「这条红屏是不是缺原生模块」要看 Stack 里有没有
   `requireNativeModule` / `<global> (.../index.js:1)`。**

## 正确姿势

```ts
import { requireOptionalNativeModule } from "expo";

const NATIVE = "ExponentAccelerometer";           // 包内部用的模块名（不是包名）
if (requireOptionalNativeModule(NATIVE) == null) {
  return null;                                    // 缺模块：静默降级，绝不 require 那个包
}
const { Accelerometer } = require("expo-sensors"); // 探测过了才碰它
```

- `requireOptionalNativeModule(name)` 缺失时返回 `null` **不抛**（`react-native` 的
  `TurboModuleRegistry.get` 同款语义）；`requireNativeModule(name)` 才抛。
- 模块名要去包源码里确认：`expo-sensors/build/ExponentAccelerometer.js` 里写的是
  `requireNativeModule('ExponentAccelerometer')`，而**不是** `ExpoAccelerometer`。
- 每加一个原生依赖，记得给「旧基包」留降级路径（或明确要求重出基包）：Luka 的
  `use-shake.ts` 就是模板；`services/good-review.ts` 对 `ExpoStoreReview` 也是同一招。

## 相关

- [Metro bundle 热 + 文件写入的坑](metro-bundle-hot-and-file-write.md)
- [RN / Expo 原生导入边界](../../../../sitin-rn/docs/architecture.md)（app 侧：`tests/design/native-import-boundary.test.ts` 记的 Skia 事故）
