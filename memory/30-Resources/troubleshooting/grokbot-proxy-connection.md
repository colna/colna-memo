---
title: Grok Bot 桌面端连不上服务（Clash 关 TUN 时强制走本地代理）
date: 2026-11-24
tags: [grok-bot, cursor, proxy, clash, troubleshooting, macos]
---

# Grok Bot 桌面端连不上服务（Clash 关 TUN 时强制走本地代理）

**症状（macOS，Grok Bot 1.2.3）**

- 详情面板显示「无法加载 Bot 详情」，点重试无效。
- 「Bot 的电脑」显示「离线」，会话提示「此对话可能不是最新的，连接恢复后会自动刷新」。
- 发消息不回，`sand.send_ack` 记 `queued_offline`，等 120s/240s 才 ack。
- 重启 App 无效（16:37 重启后持续失败 12 分钟）。

**日志证据**（`~/Library/Application Support/Grok Bot/desktop-structured-log-spill.v1.json`）

15 分钟内 958 条 warn：

| 事件 | 条数 | 细节 |
|---|---|---|
| `sand.desktop.edge_failed` | 564 | `ConnectError.PermissionDenied` 458、`DeadlineExceeded` 49 |
| `sand.box_reachability` | 330 | `access_denied` 226、`network` 101、`timeout` 3 |

`base_url_kind` 恒为 `unknown`（拿不到 box 的 pod URL）。

**根因判断**

- 反查 `app.asar`：`access_denied` = HTTP 401/403（`$l(status)`：401/403 → access_denied，5xx → http_5xx，417 → pod_hibernated）。
- 网络侧：Clash Verge 运行中，**TUN 关闭**，系统代理 `127.0.0.1:7897`；但 Grok Bot 主进程**直连** `104.18.18.125:443`（api2.cursor.sh），完全没走代理。
- 同机 curl 直连 api2.cursor.sh 正常（`/` 200；未带 token 的 RPC 401），说明域名可达，不是 DNS/黑洞。
- 结论：**直连路径被拒（403）+ 部分连接异常**，与 Windows 客户端「Clash 关 TUN 连不上」同一类问题；不是账号/订阅问题（走代理后同一账号立即正常）。

**修复（实测有效）**

从终端带代理环境变量 + Chromium 代理参数重启：

```bash
osascript -e 'quit app "Grok Bot"'
open -a "Grok Bot" \
  --env SAND_SYSTEM_PROXY=proxy \
  --env SAND_HTTPS_PROXY=http://127.0.0.1:7897 \
  --env HTTPS_PROXY=http://127.0.0.1:7897 \
  --env HTTP_PROXY=http://127.0.0.1:7897 \
  --env ALL_PROXY=http://127.0.0.1:7897 \
  --env NO_PROXY=127.0.0.1,localhost \
  --args --proxy-server=http://127.0.0.1:7897 --disable-quic
```

验证：

- 进程连接全部走 `127.0.0.1:7897`（`lsof -a -p <pid> -iTCP`）。
- 「无法加载 Bot 详情」消失，详情面板正常加载。
- 电脑状态：休眠中 → 运行中（点击可成功唤醒）。
- 不再出现 `access_denied`。

**两个网络栈都要管**

- Node/fetch 层：认 `SAND_HTTPS_PROXY` / `HTTPS_PROXY` / `ALL_PROXY`（asar 里有明确的报错文案 "If you are behind a corporate proxy, set the HTTPS_PROXY environment variable"）。
- Chromium 层：认 `--proxy-server`（系统代理一般够用，但显式给更稳）。
- 只设 env 或只加 flag 可能只修一半。

**其它**

- `SAND_SYSTEM_PROXY` 取值：`proxy` / `direct` / `auto`（默认 auto）；`SAND_HTTPS_PROXY` 需是 `http(s)://host:port` 且无路径。
- 从 Dock/Finder 启动不会继承 shell 环境变量；要么用上述命令启动，要么把变量写进 login 环境（`launchctl setenv` 或 LaunchAgent）才能持久。
- app 版本 1.2.3（Electron 43.8.9），2026-11-18 构建。
