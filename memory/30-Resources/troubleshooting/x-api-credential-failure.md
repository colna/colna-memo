---
title: X API 凭据层失效：OAuth2 invalid_client + OAuth1 401
date: 2026-11-06
tags: [troubleshooting, x-api, oauth, x-operator, credentials]
---

# X API 凭据层失效：OAuth2 invalid_client + OAuth1 401

## 现象

- 定时发布任务从某一刻起连续失败，`metrics` 先挂、`post` 后挂。
- OAuth2 refresh 返回 `invalid_client`；同一套凭据的 OAuth1 `v2.me` 返回 401。
- 很容易误判为「refresh token 过期」，反复重新授权无效。

## 根因

`invalid_client` 是**客户端身份**被拒，不是用户 token 过期；OAuth1 同时 401 说明问题在 app 凭据层（app 被 suspend / 删除、Client ID/Secret 被重置、权限被收回），而不是单个 token。只换 token 不可能恢复。

## 修法

1. 去 developer.x.com 确认 app 状态与 Client ID / Secret 是否仍有效。
2. 重新生成凭据并重新走 OAuth 授权；把新值写入项目 `.env`，不要在聊天/日志里回显。
3. 用 `npm run doctor` 或等价脚本做一次端到端探针：refresh 成功 + `v2.me` 返回目标账号 + 权限含 `tweet.write`。
4. 若 OAuth2 与 OAuth1 仍同时失败，按 app 级封禁处理，不要继续重试发布。

## 判据

- 先确认发布管道活着（最近一次成功时间、失败起点、凭据探针），再看内容与分发。管道死了，内容优化没有意义。
- `invalid_client` + 401 同时出现 = 凭据层；只有 OAuth2 refresh 401 而 OAuth1 正常 = 更可能是 token 单点问题。

## 来源

2026-11-05 x-operator 审计：post 最后成功 01:30 UTC，metrics 从 03:18 UTC 起连续失败；OAuth2 `invalid_client` 与 OAuth1 401 并存。项目页 `10-Projects/x-operator.md`、`10-Projects/colna-x.md`。
