---
title: 国内环境 reqwest 拉 Google JWKS 失败导致 ID token 校验失败
date: 2026-11-18
tags: reqwest, google, oauth, proxy, rust, troubleshooting
---

# 国内环境 reqwest 拉 Google JWKS 失败导致 ID token 校验失败

**症状**：手机端 Google 登录的账号选择弹窗正常、能拿到 ID token，但服务端返回 401 `invalid google token`；服务端日志：

```
google id token rejected error=could not fetch google certificates: error sending request for url (https://www.googleapis.com/oauth2/v3/certs)
```

**根因**：本机（国内网络）直连 Google 超时。macOS 系统代理在 `127.0.0.1:7897`，但 `onia-server` 进程没有代理环境变量，reqwest 直连失败。部署在马来西亚的 `api.onia.asia` 直连 Google 正常（实测 JWKS 200），所以生产不受影响。

**排查**：

```bash
curl -sS -m 10 -o /dev/null -w '%{http_code}\n' https://www.googleapis.com/oauth2/v3/certs   # 直连 000
curl -sS -m 10 -x http://127.0.0.1:7897 -o /dev/null -w '%{http_code}\n' https://www.googleapis.com/oauth2/v3/certs  # 200
```

最小 reqwest 验证：

```rust
let rt = tokio::runtime::Runtime::new().unwrap();
rt.block_on(async {
    let c = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().unwrap();
    println!("{:?}", c.get("https://www.googleapis.com/oauth2/v3/certs").send().await.map(|r| r.status()));
});
```

`HTTPS_PROXY=http://127.0.0.1:7897` 时返回 200。

**修法**：本地启动 server 时带上代理环境变量（reqwest 12 默认启用系统/环境代理）：

```bash
HTTPS_PROXY=http://127.0.0.1:7897 HTTP_PROXY=http://127.0.0.1:7897 \
  ALL_PROXY=http://127.0.0.1:7897 NO_PROXY=127.0.0.1,localhost \
  nohup ./target/debug/onia-server > /tmp/onia-server.log 2>&1 &
```

**备注**：Android 的 Google 登录账号选择是系统 Credential Manager 底部弹窗（原生），不是浏览器跳转；这是当前推荐流程。浏览器跳转属于 OAuth authorization code 流程。
