---
title: 真机性能排查：把 Metro 日志当测量通道（RN / Expo dev client）
date: 2026-09-17
tags: [react-native, expo, debugging, performance, metro, luka]
---

# 真机性能排查：把 Metro 日志当测量通道

**场景**：模拟器上不慢、真机（老设备 + Metro dev 包）慢；想量「点一下到页面出现」到底是几毫秒、花在哪，但手机不方便插线、Instruments 也连不上（见文末坑）。

## 一句话结论

dev client 上的 `console.log` 会**结构化**写进 Metro 的日志文件（含毫秒时间戳），在真机上打点、在这台 Mac 上读文件，就能做毫秒级真机测量 —— 不需要插线。

## 日志在哪、长什么样

`apps/<app>/.expo/dev/logs/start.log`（Metro 每次启动往里追加；几十 MB 很正常）：

```json
{"_e":"metro:client_log","_t":1789617008504,"level":"info","data":["[PERF-ENTRY] chat-press", "1789617008422", "2100034799"]}
```

- `_t`：Metro **收到**该行的时间（ms）—— 可以当钟用（与设备时钟差几十毫秒，需对齐就取同一行的第二个字段）；
- `data`：`console.log` 的各个参数原样保留，所以**把时间戳作为参数打进去**，不要只拼字符串。

读最新 N 条：

```bash
tail -c 200000 apps/luka/.expo/dev/logs/start.log | grep -a "PERF-"
# 或用 python 解析出 data[1]（自打的毫秒时间戳）后排序、算 delta
```

## 打点建议（一次进场拆五点）

| 点 | 打在哪 |
|---|---|
| 按下 | 行组件 `onPress` 第一行 |
| push 后 | `router.push(...)` 之后一行（两者相差通常 1ms，说明跳转没有被 JS 挡） |
| 新页首帧 | 屏幕组件函数体第一行 |
| 首屏 commit | 挂载 `useEffect` |
| 数据落地 / 过渡结束 | 数据 store 写入处 / `transitionEnd` 监听 |

点上后再补一个「手指 → JS 处理」的探测（见下节），就能区分「JS 忙排队」和「渲染/数据慢」。

## 触摸延迟：用原生时间戳

`Pressable` 的 `onPressIn` 事件里 `e.nativeEvent.timestamp` 是 **boot 基准的毫秒**（与 `Date.now()` 差一个未知常数）：

```tsx
onPressIn={(e) => console.log("[PERF-TOUCH]", Date.now(), e.nativeEvent.timestamp)}
```

`v = Date.now() - timestamp` 里含着未知常数 + 本次延迟；**跨多次点击取 `v` 的最小值当基线**，`v - v_min` 就是各次的「手指 → JS 处理」延迟。
（≈0–80ms 属正常；出现 1s 级说明 JS 线程被占满，点击在排队。）

## 坑

- **dev 包 ≠ release**：React dev 模式、未压缩 bundle、每条 `console.log` 走 WebSocket 到 Metro —— 老设备上整体 2–5 倍开销。慢的结论要拿 release/preview 包复核。
- **iPhone X (iOS 16) + Xcode 26 用不了 Instruments**：`xcdevice list` 说 available，但 `xctrace list devices` 一直列在 Offline，`--launch/--attach` 直接报 `Device is offline`。iOS 16 设备要**解锁 + 直连 USB**（无线配对不算）；Xcode 26 对老 iOS 的 on-device profiling 支持也不确定。快速替代方案就是本文的日志打点法。
- 应用自带的网络日志（如 `[接口] …`）在这份日志里也有 —— 排查时注意别把「日志本身的开销」算进业务耗时。
- 真机日志与模拟器日志**混在同一个文件里**：用账号 id / 时间窗切分（同一个 app 的两个客户端账号不同）。

## 相关

- [[biome-jsx-comment-backtick]] —— 打点时往 JSX 里塞中文注释踩到的 parse error
- [[metro-bundle-hot-and-file-write]] —— Metro 文件写入相关
