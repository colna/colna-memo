---
title: Clash TUN 挡 GitHub SSH，git 操作改走一次性 HTTPS
date: 2026-09-15
tags: troubleshooting, git, network, clash, sitin-rn
---

# Clash TUN 挡 GitHub SSH

- **现象**:`git fetch` / `git push` / `git ls-remote` 报
  `Connection closed by 198.18.0.30 port 443`(或 `198.18.0.102`),`kex_exchange_identification`
  超时;而同一时刻 `curl -sI https://github.com` 返回 200、`ssh -T git@github.com` 有时也能
  认证成功。「curl 通、git 挂」并存是典型信号。
- **位置**:`198.18.0.0/15` 是 Clash 的 fake-ip 段 —— 流量走 TUN,git 的长连接/大包传输被
  节点或规则掐断。不是 SSH key 问题,`ssh -T` 能过就是证明。
- **修法(不动 remote 配置)**:用**一次性 HTTPS URL**,凭证由 osxkeychain 提供:
  ```bash
  git fetch https://github.com/<owner>/<repo>.git <branch>:refs/remotes/origin/<branch>
  git push  https://github.com/<owner>/<repo>.git <branch>
  ```
  注意 fetch 要显式写 refspec,否则只进 `FETCH_HEAD`、不更新远程跟踪分支。
- **别做**:不要 `git remote set-url` 改成 HTTPS(AGENTS.md 明确不改 remote);
  也不要依赖 `ssh.github.com:443` 之类的映射 —— 本次实测 443 端口的 SSH 同样被掐。
- **环境侧(用户操作)**:Clash Verge 里切节点,或让 GitHub 走直连规则。TUN 模式下
  没有本地 HTTP 代理端口(`7890` 等都不监听),想绕 TUN 必须先关 TUN 或改规则。
- **复现记录**:2026-09-14 推送(`eedeea12`)、2026-09-15 拉取 blueprint 分支,同一症状
  两次;两次都靠一次性 HTTPS URL 解决。
