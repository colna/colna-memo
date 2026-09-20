---
title: macOS 磁盘空间排查与安全清理
date: 2026-09-20
tags: [troubleshooting, macos, disk, uv, pnpm, xcode, simulator]
---

# macOS 磁盘空间排查与安全清理

## 核心结论

- 看真实剩余空间用 `df -h /System/Volumes/Data`，不要只看 Finder「已使用」。
- Finder「已使用」比 `diskutil` 的 Volume Used 多约 58G 时，多为 purgeable（可清除），非真实占用。
- 逐层下钻用 `du -sh` 不需要 sudo，优先查 `~/Library`、`/Library/Developer`、`~/.cache`、`~/.gradle`。

## 高收益安全清理项（实测数据 2026-09-20）

- `uv cache clean`：清 21.1GiB / 185 万文件。
- `rm -rf ~/Library/Developer/Xcode/DerivedData/*`：27G → 3G。
- pnpm store：`rm -rf ~/Library/pnpm/store/v3` + `pnpm store prune`；pnpm 10 在 macOS 用 clonefile，prune 后 store 清空但 node_modules 完好。
- `rm -rf ~/.gradle/caches`：7.5G → 732M。
- `rm -rf ~/Library/Caches/*`：24G → 523M（`dotslash` 521M 需权限没删掉）。
- iOS 模拟器：`xcrun simctl delete`；Android AVD：删 `~/.android/avd/<device>` + `system-images`。

## 坑

- Docker.raw 稀疏文件：`~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw` 逻辑 460G，实际仅 15M，`du` 看真实占用别被逻辑大小骗。
- 单文件死日志：`~/.openclaw/logs/gateway.log` 曾达 31G 且进程已停；删除前确认进程不运行、目录里无凭据（如 feishu-pairing.json）。
- 权限不足跳过的：`/Library/Developer/CoreSimulator/Caches`、`/Library/Updates`、`~/Library/Caches/dotslash`，不强求 sudo。

## 删除前的确认清单

- 目标目录是否还有运行中进程（`ps -ef | rg <name>`）。
- 是否含 git 仓库 / sqlite / 凭据文件，需单独确认。
- 真删永远 `rm` 对象明确到路径，不用通配删整个不熟目录。
