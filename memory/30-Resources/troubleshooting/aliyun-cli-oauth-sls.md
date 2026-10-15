# aliyun-cli OAuth 登录（查 SLS 日志用）

> 沉淀时间：2026-10-15 · 场景：本机用阿里云 CLI OAuth 登录，供 social-proxy-agent 查 SLS 日志

## 结论速记

- **公司账号在国际站（INTL）**：控制台 `*.alibabacloud.com`、SLS region `us-west-1`。
- 登录成功命令（本机实测可用）：
  ```bash
  # 交互运行，站点提示处输 1（INTL）；非交互场景可 stdin 预喂：
  printf '1\n' | aliyun configure --mode OAuth --profile zuoyongjie
  # 完成后补 region（OAuth 流程不会问你 region 时）：
  aliyun configure set --profile zuoyongjie --region us-west-1
  ```
- 验证：`aliyun ecs DescribeRegions --profile zuoyongjie`（能返回地域列表 = 凭证 OK）。
- 身份确认：`aliyun sts GetCallerIdentity --profile zuoyongjie`（RAM user zhangzheng / 主账号 5919561568029941）。

## 坑 1：报「应用授权权限不足，请联系您的云账号管理员」= 站点选错

- **现象**：浏览器打开授权链接，点授权后提示应用授权权限不足。
- **根因**：CLI 的站点提示默认选 `0`（CN，阿里云中国站）。后台/非交互运行时提示没输入 → 默认 CN → 生成的是**中国站**授权链接（`signin.aliyun.com`），但公司账号在**国际站**，CN 站的 official-cli 应用对该账号不可授权/权限不足。
- **判据**（看授权域名即可）：
  - `signin.alibabacloud.com` = 国际站 ✅（公司账号选这个）
  - `signin.aliyun.com` = 中国站 ❌
- **修法**：在 `OAuth Site Type (CN: 0 or INTL: 1, default: CN)` 提示处选 `1`；非交互用 `printf '1\n' | aliyun configure --mode OAuth --profile <名>` 预喂 stdin。
- **附**：`--oauth-site-type INTL` 参数在 3.5.1 的 `configure` 流程**不生效**（实测仍走默认 CN）；CLI 版本 3.5.1（brew 安装）。

## 坑 2：`ERROR: region can't be empty`

- **现象**：OAuth 配好后调 API 报 region 为空。
- **根因**：OAuth 流程（非 TTY）不会问 Default Region，profile 里 region 缺失。
- **修法**：`aliyun configure set --profile <名> --region us-west-1`。

## 坑 3：STS 只有 3 小时有效期（集成时必读）

- OAuth 拿到的是**临时 STS 凭证**（`~/.aliyun/config.json` 里 `access_key_id`/`access_key_secret`/`sts_token` + `sts_expiration`）。
- 过期后**任意一次 aliyun API 调用会自动用 refresh token 换新并写回 config.json**（挑最轻的 `aliyun sts GetCallerIdentity` 当刷新触发器即可，实测 ~1.6s）。
- 集成做法（social-proxy-agent `loadAliyunOAuthCreds()`）：每次查日志前读 `sts_expiration`，临期（<5min）就跑一次 GetCallerIdentity 触发刷新，再重新读 config.json。

## 文档出处

飞书 wiki《Aliyun cli配置oauth登录》：https://presence.feishu.cn/wiki/RW7Uwr57ZiunYIkgbB0cYnPAnIe
（内嵌表格即站点对照表：0=CN aliyun.com / 1=INTL alibabacloud.com）
