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

## 同类：GitLab SSH 22 经 Clash SOCKS 被远端关闭（2026-11-02）

- **现象**：`git pull` / `git fetch` 直连 SSH 失败；经 Clash `127.0.0.1:7897` SOCKS 时 `gitlab.sitinai.com:22` 超时、**在 KEX 阶段被远端关闭**，换 2222 / 22222 同样失败。同一时刻 GitLab HTTPS 443 正常，osxkeychain 已缓存凭据。
- **根因**：网络层——代理侧掐掉 22 端口的 SSH 握手，与仓库权限 / key 无关。与本页 GitHub TUN 那例同一族（一个挡 443 长连接、一个挡 22 握手），区别是这次没有 TUN、只有本地 SOCKS 端口。
- **修法（同样「一次性 HTTPS，不改 remote」）**：
  ```bash
  git fetch https://gitlab.sitinai.com/frontend/sitin-next.git feature/pwa
  git merge --ff-only FETCH_HEAD
  git update-ref refs/remotes/origin/feature/pwa FETCH_HEAD   # 一次性 URL 只写 FETCH_HEAD，远程跟踪分支要手动同步
  ```
- **`ls-remote` 要显式绕代理**：本机 7890 已 closed、7897 是 SOCKS，环境里残留的 proxy 变量会让 HTTPS 也走代理：
  ```bash
  git -c http.proxy= -c https.proxy= ls-remote https://gitlab.sitinai.com/frontend/sitin-next.git
  ```
- **判据**：`ssh -T` 或 HTTPS 能通、`git fetch` 不通 → 先怀疑代理端口策略，别改 key、别 `remote set-url`。
- **来源**：2026-11-02 Daily 17:05 / 21:03 / 21:32 三条工作日志（fetch+ff-only merge、ls-remote 绕代理、HTTPS push 建 MR 均实跑）。
