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

- `find "/Library/Application Support/Todesk/dumps" -maxdepth 1 -type f -name '*.dmp' -delete`：2026-10-28 实测 149,710 个崩溃转储占 132G；目录 777，无需 sudo。注意清理后 CrashReport 进程仍持续写入，需另行修复 ToDesk。
- `rm -rf ~/Library/Developer/Xcode/DerivedData/*`：27G → 3G。
- `rm -rf ~/Library/Developer/Xcode/iOS\ DeviceSupport/*`：20G，连设备时会重建。
- pnpm store：`rm -rf ~/Library/pnpm/store/v3` + `pnpm store prune`；pnpm 10 在 macOS 用 clonefile，prune 后 store 清空但 node_modules 完好。
- `rm -rf ~/.gradle/caches`：7.5G → 732M。
- `rm -rf ~/Library/Caches/*`：24G → 523M（`dotslash` 521M 需权限没删掉）。
- iOS 模拟器：`xcrun simctl delete`；Android AVD：删 `~/.android/avd/<device>` + `system-images`。

## 坑

- Docker.raw 稀疏文件：`~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw` 逻辑 460G，实际仅 15M，`du` 看真实占用别被逻辑大小骗。
- 单文件死日志：`~/.openclaw/logs/gateway.log` 曾达 31G 且进程已停；删除前确认进程不运行、目录里无凭据（如 feishu-pairing.json）。
- ToDesk 会无限堆积崩溃转储：`/Library/Application Support/Todesk/dumps/*.dmp` 单文件约 916K，可能累积十几万个。清理后 CrashReport 仍在写（10 分钟新增 ~38 个），根因要修 ToDesk 本身，不是只删文件。
- `~/.npm/_cacache` 里可能有 root 属主的旧缓存文件（sudo npm 留下的），普通用户删不掉；`npm cache clean --force` 也会失败，残留 1G 级别可接受，别为它上 sudo。
- 权限不足跳过的：`/Library/Developer/CoreSimulator/Caches`、`/Library/Updates`、`~/Library/Caches/dotslash`，不强求 sudo。

## ToDesk 卸载（2026-10-28）

- 官方机制：`/Library/LaunchDaemons/com.youqu.todesk.UninstallerWatcher.plist` 监听 `/Applications/ToDesk.app`，app 消失后自动以 root 运行 `ToDeskUninstallerStarter --module Full`，清理 LaunchDaemons/LaunchAgents、PrivilegedHelperTools、Audio HAL 驱动。
- 直接 `rm -rf /Applications/ToDesk.app` 会因 bundle 内 `root:admin 755` 目录失败；用同卷 `mv /Applications/ToDesk.app ~/.Trash/ToDesk_removed.app` 即可触发 Watcher，无需 sudo。
- 卸载器退出后残留 `/Library/Application Support/Todesk/dumps`（期间新生成的 dump）和 `/Library/Logs/ToDesk`（root:wheel 700），需要一次管理员授权删除。
- `~/.Trash` 受 TCC 保护，普通 shell 甚至 osascript root 都是 `Operation not permitted`；用 Finder AppleScript：`tell application "Finder" to delete item "xxx" of trash`。
- Core Audio Driver 由 `_coreaudiod` 持有，驱动文件删除后进程仍在，重启/注销后消失，不必强杀。
- `~/Library/Preferences/com.todesktop.230313mzl4w4u92.*` 是 ToDesktop 打包 app 的通用 ID，未必属于 ToDesk，别乱删。

## 删除前的确认清单

- 目标目录是否还有运行中进程（`ps -ef | rg <name>`）。
- 是否含 git 仓库 / sqlite / 凭据文件，需单独确认。
- 真删永远 `rm` 对象明确到路径，不用通配删整个不熟目录。
