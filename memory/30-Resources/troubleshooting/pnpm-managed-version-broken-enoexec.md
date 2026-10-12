---
title: pnpm 受管版本本地 tool 损坏 → spawnSync ENOEXEC
date: 2026-10-12
tags: troubleshooting, pnpm, macos, node
---

# pnpm 受管版本本地 tool 损坏 → spawnSync ENOEXEC

## 现象

进入 pin 了 `packageManager: pnpm@<v>` 的仓库（pnpm 的 manage-package-manager-versions 会
自动切到该版本），任何 pnpm 命令直接报：

```
ERROR Failed to switch pnpm to v12.4.2. Looks like pnpm CLI is missing at
"/Users/<user>/Library/pnpm/.tools/pnpm/12.4.2/bin" or is incorrect
spawnSync /Users/<user>/Library/pnpm/.tools/pnpm/12.4.2/bin/pnpm ENOEXEC
```

## 根因

受管版本装在 `~/Library/pnpm/.tools/pnpm/<v>/`，但它的 postinstall（`install.js`，负责把
host native binary 链接到 `node_modules/pnpm/pnpm`）没跑完 —— 目录里残留 `_tmp_*`，
`pnpm` 还是那个**故意无 shebang** 的 placeholder：

- 人手动在 shell 里敲 `.../bin/pnpm --version` 是能跑的（zsh/fish 在 execve ENOEXEC 后
  会兜底用 shell 执行，placeholder 内部再转 Node）；
- 但 pnpm 父进程用 Node `spawnSync` 直接 execve → ENOEXEC，没有 shell 兜底，于是报错。

placeholder 无 shebang 是设计如此（注释见该文件头部），所以别去给它加 shebang。

## 修法

直接补跑安装脚本，不需要删目录也不需要重装 pnpm：

```bash
node ~/Library/pnpm/.tools/pnpm/<v>/node_modules/pnpm/install.js
```

## 验证

```bash
file ~/Library/pnpm/.tools/pnpm/<v>/node_modules/pnpm/pnpm   # 应为 Mach-O arm64
cd <目标仓库> && pnpm -v                                      # 输出 <v>
```

## 备注

- 出现 `_tmp_*` 兄弟目录 = 之前有一次下载/安装中断。
- 直接删掉损坏目录再跑一次 `pnpm -v` 让 pnpm 重新下载也可行，但要先确认目录里没有
  其它版本残留（本机 .tools/pnpm 下一堆历史版本，别一把梭）。
