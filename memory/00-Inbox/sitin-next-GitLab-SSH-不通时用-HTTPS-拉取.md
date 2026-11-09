---
title: "sitin-next GitLab SSH 不通时用 HTTPS 拉取"
date: 2026-11-09
tags: sitin-next,gitlab,git,ssh,troubleshooting
---

## 背景

仓库：`sitin-next`（remote：`git@gitlab.sitinai.com:frontend/sitin-next.git`）
分支：`feature/pwa`
日期：2026-11-09

## 现象

- `git pull`（SSH :22）失败：本机代理 `127.0.0.1:7897` 断开，或直连 `gitlab.sitinai.com:22` 超时
- `:443` HTTPS 可达；SSH over 443 也会被远端关掉

## 做法

主仓用 HTTPS fetch + ff-only merge：

```bash
git fetch https://gitlab.sitinai.com/frontend/sitin-next.git feature/pwa
git merge --ff-only FETCH_HEAD
git update-ref refs/remotes/origin/feature/pwa HEAD
```

子模块 `packages/business-pwa-proto/proto`（`git@gitlab.sitinai.com:packages/proto.git`）同样走 HTTPS：

```bash
EXPECTED=$(git ls-tree HEAD packages/business-pwa-proto/proto | awk '{print $3}')
git -C packages/business-pwa-proto/proto fetch https://gitlab.sitinai.com/packages/proto.git "$EXPECTED"
git -C packages/business-pwa-proto/proto checkout --force "$EXPECTED"
```

## 结果

- `feature/pwa`：`845a2ed7f` → `0207334ae`（Merge branch `feat/gxy/delsitin`）
- proto 子模块：`d77e5ec6`

## 注意

未改 remote URL；SSH/代理修好后可继续用原 SSH remote。
