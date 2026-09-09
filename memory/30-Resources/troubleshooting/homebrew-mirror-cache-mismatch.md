---
title: Homebrew 排错(阿里云镜像缓存名不匹配 / link 冲突)
date: 2026-09-09
tags: troubleshooting, homebrew, macos, mirror
---

# Homebrew 排错

## `brew install` 全部包报 `rb_sysopen - No such file or directory`(2026-09-09)

- **现象**:`brew install ccache` 时 7 个包(blake3 / ca-certificates / fmt / hiredis / zstd / ccache / openssl@3)
  进度条都跑到 `Extracting 100%`,然后整齐地报
  `Error: No such file or directory @ rb_sysopen - ~/Library/Caches/Homebrew/downloads/<sha>--blake3-1.8.7.arm64_tahoe.bottle.tar.gz`。
  首行伴随 `Warning: Bottle missing, falling back to the default domain`。

- **根因**:`HOMEBREW_BOTTLE_DOMAIN` 指向阿里云镜像,镜像里没有这批 bottle → Homebrew 回退官方域名下载,
  但**缓存路径仍按镜像 URL 的 basename 推导**,于是下载存到 A、解压去开 B。
  Homebrew **6.0.10** 有此问题,6.0.22 没有。

- **判据(一眼定位,不用猜)**:看 `~/Library/Caches/Homebrew/downloads/` 的文件名横杠数
  - 正常保存的 bottle:`<sha>--fmt--12.2.0.arm64_tahoe.bottle.tar.gz`(name 与 version 间**双横杠**)
  - 报错要打开的:`<sha>--fmt-12.2.0.arm64_tahoe.bottle.tar.gz`(**单横杠**,是 manifest 的命名风格)
  - 再看时间戳:本次安装只有 `*.bottle_manifest.json` 是新的,`*.tar.gz` 全是旧日期 → tar.gz 根本没落盘。

- **修法**:
  ```bash
  env -u HOMEBREW_BOTTLE_DOMAIN -u HOMEBREW_API_DOMAIN brew install <pkg>   # 一次性绕开镜像
  brew update && brew --version                                            # 升到 6.0.22+ 再用镜像
  ```
  长期折中(API 走镜像加速、bottle 走官方):保留 `HOMEBREW_API_DOMAIN`,`unset HOMEBREW_BOTTLE_DOMAIN`。

- **排掉的错误方向**(都不是原因,别浪费时间):缓存目录权限/属主(是 `colna:staff` 正常)、
  安全软件隔离、网络。**磁盘满会导致 macOS 清理 `~/Library/Caches`,是另一条独立的因果链**,别和本条混。

- **教训**:第三方 bottle 镜像 + 新版 Homebrew 是高频不兼容组合;
  `Warning: Bottle missing, falling back to the default domain` 这句警告和后面的 `rb_sysopen` 报错是**同一件事的两面**,
  见到警告就先怀疑镜像回退,而不是去查权限。

## `Error: Cannot link ca-certificates` / `Another version is already linked`

- **现象**:上面的缓存问题解决后,pour 阶段报 `Another version is already linked: /opt/homebrew/Cellar/ca-certificates/2026-08-13`。
- **根因**:Cellar 里旧 keg 仍占着 link(常见于同版本号但 formula revision 变了,如 `2026-08-13` vs `2026-08-13-1`)。
- **修法**(从轻到重):
  ```bash
  brew unlink ca-certificates && brew install <pkg>
  brew link --overwrite ca-certificates
  brew uninstall --ignore-dependencies ca-certificates && brew install ca-certificates
  ```
  `unlink` 后到重新 link 之间,别跑依赖 TLS 证书的命令(curl / git clone)。

## `df -h /` 看到的不是真实剩余空间

- `/dev/disk3s1s1` 是**只读系统快照卷**,`Used` 只有 16Gi 但 `Avail` 是整个 APFS 容器的共享可用空间。
- 看数据卷要用 `df -h /System/Volumes/Data`。
- 磁盘紧张时 macOS 会主动清理 `~/Library/Caches`,后续会引发各种莫名其妙的缓存失效。
