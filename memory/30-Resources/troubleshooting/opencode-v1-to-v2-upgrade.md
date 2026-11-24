---
title: OpenCode V1 升 V2 与手机 pair
date: 2026-11-24
tags: [opencode, macos, troubleshooting]
---

# OpenCode V1 升 V2 与手机 pair

## 现象

- `opencode pair --remote` 报错 / 只打印帮助，提示没有 `pair` 子命令。
- 桌面端 App 内「检查更新」一直停在 1.19.6，明明 v2 已发布。

## 根因

1. `pair`（含 `--remote`）是 **OpenCode V2** 功能；V1（npm `opencode-ai` 1.19.6）没有 `service` / `pair` 子命令。
2. 桌面端自动更新走 electron-updater 的 GitHub provider + `channel: latest` → 实际请求 GitHub `releases/latest`，而该仓库把 **v1.19.6 标为 latest**（v2.4.x 虽已发布但未设 latest），所以永远收不到 v2。
3. 官方 `https://opencode.ai/update/api/latest/desktop/opencode/` 返回的才是 2.4.1，与 GitHub 的 latest 不一致。

## 修法

### 桌面端（macOS）

```bash
osascript -e 'quit app "OpenCode"'
mv /Applications/OpenCode.app /Applications/OpenCode-v1.19.6.app   # 回滚备份
curl -fL -o /tmp/oc.dmg https://opencode.ai/files/bin/2.4.1/opencode-desktop-mac-arm64.dmg
shasum -a 256 /tmp/oc.dmg   # 与更新 API JSON 的 sha256 比对，不能只看文件名
hdiutil attach -nobrowse -readonly /tmp/oc.dmg
ditto /Volumes/OpenCode/OpenCode.app /Applications/OpenCode.app
hdiutil detach /Volumes/OpenCode
```

校验：`defaults read .../Info.plist CFBundleShortVersionString`、`codesign -dv`（应 `ai.opencode.desktop` / Anomaly Innovations）、`spctl -a -vvv -t exec` 应 `Notarized Developer ID`。

### CLI

- `npm i -g @opencode/cli`（2.4.x）会同时提供 `opencode` 和 **`opencode2`** 两个 bin；想保留 v1 就只用 `opencode2`。
- 或 `brew tap anomalyco/tap && brew install opencode-v2`（二进制 `opencode-v2`）。
- 官方声明 v1/v2 不再默认并存，curl 安装器会直接替换 v1 二进制。

### 手机配对

```bash
opencode2 pair            # 同一局域网优先；默认只听 127.0.0.1
opencode2 pair --remote   # 走 OpenTunnel 暴露公网，首次需确认，非交互加 --yes
```

- 打印一次性链接 + 二维码，5 分钟过期；手机 OpenCode app 扫码或填 server 地址。会话 30 天。
- `pair` 依赖共享后台服务；`opencode service set disabled true` 后不可用。
- 同网段更安全：`opencode service set hostname 0.0.0.0`，再 `pair`，用局域网 IP 访问。

## 注意

- v1/v2 共用 `~/.config/opencode` 与本地数据库，v2 首次启动会迁移配置/会话，之后 v1 不一定能读。
- v2 插件 API 是破坏性变更，v1 插件不生效。
- v2 不再默认拦截项目外路径（`external_directory` 默认 allow）。
