---
title: Luka「退出登录闪退 + 账号消失」：JS fatal（RCTFatal abort）撞上「先清凭证再登出」
date: 2026-09-21
tags: troubleshooting, react-native, ios, crash, ips, rctfatal, logout, luka
---

# Luka「退出登录闪退 + 账号消失」：JS fatal 与「先清凭证」叠加

**症状**（2026-09-29，Luka 1.0.0 内测包 `com.presence.gracechat`，iPhone X / iOS 16.7.16）：
设置页点「Sign out」后**偶现闪退**；重启后账号「被删了」——落到登录页，重新登录得到一个全新账号。
测试同学提供 session.log + 3 份 .ips（`~/TEST`）。

## 一、先把 .ips 和日志对上号（时区差是坑）

- session.log 的时间戳是 **UTC**；.ips 的 `timestamp` 是**设备本地（+0800）**，差 8 小时。
- 三份 .ips 与 session.log 里三次「日志中断 + 数秒后 `luka_app_launch`」一一对应：

| .ips（+0800） | session.log（UTC） | 场景 |
| --- | --- | --- |
| 11:08:53 | 03:08:48–53 | dev 页附近（无 `luka_logout_click`，疑似 Debug Tools 的 Reset device identity，走同一条登出链） |
| 11:29:59 | 03:29:51–56 | 订阅返回后刮层（= react-native-svg mask crash，见 [[react-native-svg-ios-mask-crash]]） |
| **13:54:57** | **05:54:50–57** | **设置页点退出登录** |

## 二、登出崩溃是「JS fatal」，不是普通 native crash

`Luka-2026-09-29-135457.ips` 的 `last_exception_backtrace`（自内向外）：

```
objc_exception_throw
← RCTGetFatalHandler
← -[RCTExceptionsManager reportFatal:stack:exceptionId:extraDataAsJSON:]
← -[RCTExceptionsManager reportException:]
← invocation function for block in ObjCTurboModule::performVoidMethodInvocation(...)
（faulting thread: com.meta.react.turbomodulemanager.queue）
```

- 语义：**JS 侧有未捕获的 fatal 错误** → RN 的 `ErrorUtils` 全局 handler → `ExceptionsManager.reportException(fatal)` → `RCTFatal` 的 `@throw` → 进程 abort（SIGABRT）。
- `110853` 是同一队列、同一 abort（栈形态为 `std::terminate → _objc_terminate`）——两次崩溃同源的可能性很高。
- session.log 里**一条 error 都没有**：fatal 路径不会给异步日志任何落盘机会。这既是为什么现场查不到 JS 栈，也是为什么必须加「遗言」（见四）。

## 三、「账号被删」的真相：不是删除，是「换身份重登 = 后端开新号」

全日志零 DeleteUser 请求，后端账号还在。链路：

1. 崩溃发生在 `logout()` 已经把本地会话清掉之后（当时的实现先 `clearSession()`）→ 重启落登录页；
2. iOS 登录页只有 Apple 入口（device 号没有任何入口）→ 用户用 Apple 登录；
3. 后端 `FastLogin`（`packages/business-auth/src/auth.ts`：设备登录按 `deviceId` 认人；Apple 入口把 **Apple user id 当 deviceId**）——与 device 号的绑定对不上 → **mint 一个新账号**。

日志实证（userId 随身份走）：

| 时刻 | 动作 | 结果 |
| --- | --- | --- |
| 03:08 崩溃 A 后 | **自动** quick_login（`luka.66bb4d3d…` + googleEmail，无 `sign_in_click`） | 新号 `2100067632` "Fh"（上一号 `2100067621` "Hhi" 从此无入口） |
| 05:54 登出崩溃 C 后 | Apple 登录（`001384.ed42db…0315`，login_type=apple） | 新号 `2100067642` "Fy" |
| 05:57 再登出、再 Apple 登录 | 同 Apple ID | 仍 `2100067642` "Fy"（绑定一旦建立就稳定） |

所以「偶现删号」= 崩溃时机 × 先清凭证 × 换身份重登，三者叠加；对用户就是「号没了」。

## 四、修了什么（客户端加固，不依赖根因）

| 修复 | 文件 | 效果 |
| --- | --- | --- |
| teardown 逐条隔离 + 面包屑 | `services/native/logout-teardown.ts`（新）；`bootstrapOnLogout` 全量套上 | 一步抛错不再跳过后续（原先同步段是裸调用，且异常被调用方的空 catch 吞掉）；崩溃时日志最后一条面包屑就是死掉的那一步 |
| 登出顺序：**凭证最后删** | `stores/auth.ts` | 崩在登出中途时磁盘会话仍完整，重启恢复同一账号；原先先 `clearSession()`，任何一步崩都把用户扔到登录页 |
| JS fatal 遗言 | `services/native/js-fatal-breadcrumb.ts`（新） | 包 `ErrorUtils` 全局 handler：fatal 先按 `[CrashLog]` **同步**写 crash.log（含 JS 栈），再原样交回原 handler；下次闪退「Export crash log」能导出真正的 JS 错误 |
| 登出链路面包屑 | `app/settings/logout.tsx` | 确认点击 / chat teardown / auth logout 各一条 |

回归：`tests/services/logout-teardown.test.ts`（4）、`tests/stores/auth-logout.test.ts`（3，含「凭证清除前抛错时不得清凭证」）、`tests/services/js-fatal-breadcrumb.test.ts`（4）。

## 五、遗留 / 待定夺

- **崩溃根因**（到底哪个 JS 错误）：等带遗言的包复现后，从 Debug Tools →「Export crash log」取 `[CrashLog]` 条目。嫌疑方向：登出触发的原生 SDK 回调（TIM / BytePlus / AppsFlyer / TRTC 在 teardown 时同步 emit 事件，JS listener 访问已清状态）——但没有栈之前不下结论。
- **iOS 登出后必然开新号**属产品/后端决策：登录页默认回放上次登录方式，或后端按可验证身份（Apple id / email）归并账号。
- 测试包与 `main` 有差异（03:08 的自动 quick_login 带 `googleEmail`，而 `main` 的 iOS 登录页没有 Google 入口）——以后查崩溃先确认构建版本。

## 附：.ips 判读速查

```bash
IPS=~/TEST/Luka-xxx.ips python3 - <<'PY'
import json, os
with open(os.environ["IPS"]) as f:
    header, body = json.loads(f.readline()), json.loads(f.read())
print("ts:", header.get("timestamp"), "| bundle:", header.get("bundleID"))
print("exc:", body.get("exception", {}).get("type"), body.get("exception", {}).get("signal"))
print("termination:", body.get("termination", {}).get("indicator"))
for fr in (body.get("last_exception_backtrace") or [])[:12]:
    print("  ", fr.get("symbol"))
ft = body["threads"][body.get("faultingThread", 0)]
print("faulting:", ft.get("name") or ft.get("queue"))
for fr in ft.get("frames", [])[:12]:
    print("   ", fr.get("symbol"))
PY
```

- `last_exception_backtrace` 有 `RCTExceptionsManager reportFatal` → **JS fatal**（去装 fatal 遗言，别在 native 侧瞎找）。
- `faultingThread` 队列名是重要线索：`com.meta.react.turbomodulemanager.queue` = 某次 TurboModule 调用链上出事。
- 与 session.log 对照时记得 **.ips 用设备时区、日志可能是 UTC**。
