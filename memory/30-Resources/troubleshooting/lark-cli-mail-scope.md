# lark-cli 读飞书邮件授权踩坑

## 2026-08-14 读 zhangzheng@heyhru.com 收件箱:邮件内容读 scope 拿不到

**目标**:用 lark-cli 读某人收件箱(mail 域)。**结论:被敏感权限管控卡住,未成功。**

**排查链路(可复用)**
1. `auth status --json --verify` → `identities.user.userName` 看当前登录是谁;`me` 邮箱用 `mail user_mailboxes profile --params '{"user_mailbox_id":"me"}'`。**user 身份只能读自己邮箱**("me"),读别人要管理员级 scope。
2. 本工作区默认 app `cli_a95cafcf9cb9dcd4`(张峥);同 app user token 只存一个,**换人登录会覆盖**(步川→张峥)。config 在 `~/.lark-cli/config.json`(`apps[]`)。
3. 读收件箱(`+triage` / `+message`)需 4 个 scope:`mail:user_mailbox.message:readonly / .address:read / .subject:read / .body:read`。
4. 授权用 split-flow:`auth login --domain mail --no-wait --json` 拿 `verification_url`+`device_code` → `auth qrcode <url> --output x.png`(相对路径)发用户扫 → 用户扫完 `auth login --device-code <code>` 完成。

**坑:开通了权限仍「本次新授予:空」**
- 光在权限管理页勾 scope 不够,要**创建版本→发布→审批**。
- 但即使"已发布",这 4 个**读邮件内容**scope 仍授不下来;而 `message:modify`、`mail:user_mailbox:readonly` 能授。→ 强烈怀疑读正文/主题/地址是**飞书敏感权限**,需**管理后台企业管理员单独审批数据权限**,或该应用类型不开放邮件内容读取。授权页会提示具体原因(未采集到)。
- CLI 会提示"请勿持续重试",别循环刷授权。

**下一步(待验证)**:让扫码人看授权页对这 4 项的提示原文;走管理后台「敏感权限/数据权限」审批。
