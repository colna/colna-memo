---
title: vitest / resetModules 后顶层 import 的 proto 类身份失效
date: 2026-09-16
tags: [troubleshooting, vitest, sitin-rn, protobuf, testing]
---

# `vi.resetModules()` 之后，不能再拿顶层 import 的 proto 类做身份比较

**坑**（2026-09-16，`apps/luka/tests/services/swipe-limit.test.ts`）：测试用
`vi.resetModules()` 隔离被测模块的内存状态（`authoritative` 这类模块级变量），同时在文件顶部
`import { UserApi } from "@heyhru/business-pwa-proto"`，mock 里用
`req === UserApi.QueryUserMiscRequest` 分发响应。

结果：`resetModules()` 清空模块注册表后，动态 `await import("@/services/swipe-limit")`
拿到的是**重新加载的一份** `business-pwa-proto`，而测试顶部的 `UserApi` 还是旧实例 ——
`$type` 相同的两个 Request 类不是同一个引用，`===` 为 false，分支静默走错（表现是
`call.mock.calls.find(...)` 返回 `undefined`，断言报「收到 undefined」，离根因很远）。

**修法**（二选一）：

```ts
// A. 两边用同一次加载：先 reset，再动态 import proto，最后 import 被测模块
beforeEach(() => vi.resetModules());

it("...", async () => {
  const { UserApi } = await import("@heyhru/business-pwa-proto");
  const { requestDailyNumbers } = await import("@/services/swipe-limit");
  // 此后 req === UserApi.QueryUserMiscRequest 两边一致
});
```

```ts
// B. 不比类身份，比 proto 的稳定字符串
const reqType = (req: unknown) => (req as { $type?: string })?.$type;
expect(reqType(req)).toBe("UserServiceProto.QueryUserMiscRequest");
```

**教训**：mock 里判断「这是哪个请求」永远优先用 `$type` 字符串或 payload 形状，类相等只在
**同一次模块加载**内可靠；只要测试里出现 `vi.resetModules()`，顶层 import 的类就不能再当锚点。
同类问题也适用于 Zustand store 实例、错误类（`instanceof`）等所有依赖模块身份的断言。

**相关**：`vitest.config.ts` 的注释（为什么测试跑在纯 Node：不 boot RN runtime）。
