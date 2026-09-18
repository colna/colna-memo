---
title: Xcode 登录 Apple 失败 -1200（自动签名退化到通配描述文件）
date: 2026-09-18
tags: troubleshooting, ios, xcode, provisioning, network, sitin-rn
---

# Xcode 登录 Apple 失败 -1200

## 症状

`xcodebuild ... -allowProvisioningUpdates archive` 报：

```
error: The operation couldn't be completed. Unable to log in with account '<Apple ID>'.
An unexpected failure occurred while logging in (Underlying error code -1200).
```

随后自动签名**静默退化**到 `iOS Team Provisioning Profile: *`，并连带报一串
「profile doesn't include App Groups / Push Notifications / Sign In with Apple」的
entitlements 错误 —— 看起来像证书或 pbxproj 问题，其实根因是登录没成功。

## 根因与排查

`-1200` 是 `NSURLErrorSecureConnectionFailed`，先当**网络问题**排，别先怀疑账号 / 2FA：

```bash
for u in https://idmsa.apple.com https://developerservices2.apple.com; do
  printf "direct  %s -> " "$u"; curl -s -o /dev/null -m 10 -w "%{http_code}\n" "$u"
  printf "proxy   %s -> " "$u"; curl -x http://127.0.0.1:7897 -s -o /dev/null -m 10 -w "%{http_code}\n" "$u"
done
```

本次（colna 的 Mac）：Clash Verge 监听 `127.0.0.1:7897`，规则里有
`DOMAIN-SUFFIX,apple.com,DIRECT`，`idmsa.apple.com` / `developerservices2.apple.com`
都命中直连。首查时直连与代理均 000（5s 超时），几分钟后复测全部 200 ——
属于**瞬时不可达**，原地重试即通过（不必 clean / 不用动 Xcode 账号）。

## 修法

1. 先复测 Apple 开发者域名连通性；恢复后**原命令重试**即可。
2. 持续不通时：把 `*.apple.com` 挪出 DIRECT 走代理（Clash Verge 规则 / 全局模式），
   或换到可达的网络。
3. 只有网络确认没问题、仍 -1200，再考虑 Xcode → Settings → Accounts 重新登录（2FA）。

## 记住

自动签名失败时，`error: Provisioning profile ... doesn't include ...` 这一串是
**连带症状**；先看它上面第一条 `Unable to log in with account`，否则会误朝
entitlements / pbxproj 方向排查。
