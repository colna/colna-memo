---
title: gitlab.sitinai.com 访问配置
date: 2026-10-09
tags: [gitlab, ssh, sitin]
---

# gitlab.sitinai.com 访问配置

## 结论

内网 GitLab `gitlab.sitinai.com`（账号 **zhangzheng**）已接入本机，专用 SSH key：

- 私钥：`~/.ssh/id_ed25519_gitlab_sitinai`（无 passphrase，ed25519）
- `~/.ssh/config` 已加 Host 块（`IdentityFile` 指向该 key + `IdentitiesOnly yes`）
- `~/.ssh/known_hosts` 已加 host key（ssh-keyscan）

验证：`ssh -T git@gitlab.sitinai.com` → `Welcome to GitLab, @zhangzheng!`

## 坑与流程（新内网 GitLab 接入）

1. 该 GitLab 不认本机通用 `id_rsa`（Permission denied publickey），需生成专用 key 并在 GitLab 网页注册公钥。
2. 首次连接报 `Host key verification failed` → 先 `ssh-keyscan gitlab.sitinai.com >> ~/.ssh/known_hosts`。
3. HTTPS 克隆需要 PAT（本机无存储凭据），走 SSH 更省事。
4. 本机 Clash TUN 会把域名解析到 fake-IP `198.18.x.x`，对 gitlab.sitinai.com 的 22 端口 SSH / 443 HTTPS 无影响。

## 已从这里克隆

- `~/WORK/sitin-prds`（ai/sitin-prds，PRD 静态站仓库）— 2026-10-09
