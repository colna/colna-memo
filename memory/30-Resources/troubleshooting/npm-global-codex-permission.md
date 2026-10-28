---
title: npm 全局包权限损坏修复（EACCES / ENOTEMPTY）
date: 2026-10-28
tags: [troubleshooting, npm, node, codex, permissions]
---

# npm 全局包权限损坏修复

## 现象

- `codex --version` 报 `real codex not found`（ccisland shim）或 `codex.js` 抛错。
- `npm install -g @openai/codex@latest` 报 `EACCES: rename .../node_modules/@openai/codex` 或 `ENOTEMPTY ... .codex-xxxx`。
- `npm view` / `npm outdated` 报 `EACCES: mkdir ~/.npm/_cacache/...`。

## 根因

- 之前用 root（sudo）跑过 npm，`~/.nvm/.../lib/node_modules/@openai`、`~/.npm/_cacache` 变成 root 属主。
- 失败安装留下 `.codex-<random>` 临时目录和 bin 里的 `.codex-<random>` 软链，阻塞后续 rename。

## 修法

1. 管理员授权修属主：`chown -R 501:20 ~/.nvm/versions/node/<ver>/lib/node_modules/@openai ~/.npm`
2. 清临时残留：`rm -rf <npm-root>/@openai/.codex-*`、`rm -f <prefix>/bin/.codex-*`
3. 普通用户重装：`npm install -g @openai/codex@latest`
4. 验证：`codex --version`

## 注意

- 不要用 sudo 跑 npm；全局目录保持用户属主。
- ccisland 的 codex shim 写死 `~/.nvm/versions/node/v22.22.0/bin/codex`，重装需装到该 nvm 版本。
