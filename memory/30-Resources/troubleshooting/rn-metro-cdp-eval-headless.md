---
title: 无头环境对运行中的 RN dev build 求值 JS（Metro inspector CDP）
date: 2026-10-09
tags: [troubleshooting, react-native, metro, hermes, cdp, simulator, sitin-rn]
---

# 场景

机器上没有图形化 Simulator.app（CI 式 Xcode，只有 CoreSimulator + `simctl`），但需要：
触发运行中 RN 应用的原生模块（写 App Group、调 NativeModules）、查全局对象/状态，
而 `simctl` 不能注入触摸、自定义 scheme `openurl` 又会弹「在 App 中打开?」确认框点不了。

**出路：Metro 的 Hermes inspector 本质是 CDP over WebSocket，可以从宿主 Node 直接连。**

# 步骤

```bash
# 1. 列页面，拿 webSocketDebuggerUrl（device=<logicalDeviceId>&page=<n>）
curl -s http://127.0.0.1:8081/json/list

# 2. Node（repo node_modules 有 ws）连上，发 Runtime.evaluate
#    ws://<host>:8081/inspector/debug?device=...&page=...
#    请求头 Origin 必须是 **serverBaseUrl 的 origin**（本例 http://127.0.0.1:8081）
node cdp-eval.js "<ws url>" "globalThis.expo.modules.HomeWidget.getItem('k')"
```

脚本要点（完整版在用它的会话里）：`Runtime.evaluate { returnByValue: true, awaitPromise: true }`；
**不用先 Runtime.enable**；只按 `message.id === <你的 id>` 过滤响应 —— 打开后 console 日志
（Require cycle 警告等）会刷屏。

# 坑与症状对照

| 症状 | 含义 |
| --- | --- |
| HTTP 401 `Unauthorized`（升级被拒） | `verifyClient` 没过：Origin 与 `serverBaseUrl.origin` 不等，且主机名不在白名单；**没有 Origin 头也 401** |
| 连接 open 后立刻 `close 1006` | 过了 verifyClient，但 Expo 的 `isMatchingOrigin` 对比 host 失败被 `terminate()`（例如 serverBaseUrl 是 `127.0.0.1` 而你发 `localhost`） |
| `close 1011 [PAGE_NOT_FOUND]` | **page id 变了**：应用每次 reload/重连都会把 page 号 +1（`-1` → `-2`…），去 `/json/list` 重新取 |
| 一直收到 `Runtime.consoleAPICalled` | 正常噪音，按 id 过滤 |

serverBaseUrl 的 origin 怎么确定：Expo 启动 Metro 时按本机网络选（LAN IP ↔ 127.0.0.1），
试 `http://127.0.0.1:8081`、`http://localhost:8081`、LAN IP 三个即可。

# 实践

2026-10-09 用它把 sofia 桌面组件的 App Group 快照**端到端验证**：`expo.modules.HomeWidget.setItem(...)` +
`reload()`，宿主 `plutil -p` 读 `group.<id>.plist` 确认落盘；同时真机日志里抓到真实收到的
文本消息也走通了同一条链路（无需图形界面）。

相关：[headless-screenshot-spa-cdp.md](headless-screenshot-spa-cdp.md)（浏览器侧同类思路）、
[sitin-rn-home-widget.md](sitin-rn-home-widget.md)。
