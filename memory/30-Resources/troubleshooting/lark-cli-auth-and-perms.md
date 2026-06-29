---
title: lark-cli 鉴权与文档权限踩坑
date: 2026-06-29
tags: lark-cli, feishu, troubleshooting
---

# lark-cli 鉴权与文档权限踩坑

## 1. `wiki +node-get` 报 missing scope `wiki:node:retrieve`

**现象**:
```
missing required scope(s): wiki:node:retrieve
hint: run `lark-cli auth login --scope "wiki:node:retrieve"`
```
按 hint 重新授权后,服务端依然 99991679,提示需要 `wiki:wiki / wiki:wiki:readonly / wiki:node:read`。

**根因**:CLI 的本地预检查脚本用了老 scope 名 `wiki:node:retrieve`,跟服务端 OpenAPI 实际要求的 `wiki:wiki:readonly` / `wiki:node:read` 不一致。

**修法**:不按 CLI hint,直接申请服务端要的:
```bash
lark-cli auth login --no-wait --scope "wiki:wiki:readonly wiki:node:read docx:document:readonly" --json
```

## 2. 开放平台改了 scope,本地 token 不自动刷新

**现象**:在开放平台后台勾上新 scope,CLI 依然报 `missing_scope`。

**根因**:本地缓存的 user token 是开通 scope **之前**签发的,token 里不带新 scope。开放平台改配置不会推送到已签发的 token。

**修法**:`lark-cli auth login --scope "..."` 重新走一次 device flow,新 token 会带最新已开通的全部 scope。

## 3. device-flow 在远端机器上跑

主 agent 在远端运行、用户在本地无法用我这台机器的浏览器,但 device flow 的 verification URL **可以在任何浏览器打开**(包括手机飞书 App)。

**推荐流程**(避免阻塞 + 避免 device code 失效):
```bash
# 1. 拿 code(不阻塞)
lark-cli auth login --no-wait --scope "..." --json
# → { device_code, verification_url, expires_in: 600 }

# 2. 生成二维码到 OUTPUTS_DIR(MetaBot 自动发回飞书)
lark-cli auth qrcode "<verification_url>" --output qrcode.png

# 3. 把 URL + 二维码发给用户,本轮结束;用户用手机/浏览器授权
# 4. 用户回"好了"后,主 agent 续轮询
lark-cli auth login --device-code "<device_code>"
```

**坑**:不能在同一轮发完 URL 立刻阻塞执行 `--device-code`(harness 在阻塞期间不会把消息发给用户)。每次重启 `auth login` 会作废上一轮的 device code。

## 4. docs `+update --content @file` 必须是 cwd 相对路径

**现象**:
```
--content: invalid file path "/private/tmp/.../foo.xml":
--file must be a relative path within the current directory
```

**修法**:不能传绝对路径。先 `cd` 到 scratchpad 再用 `./` 引用:
```bash
cd /private/tmp/.../scratchpad
lark-cli docs +update ... --content @./file.xml
```
shell cwd 会自动回到原目录,不影响后续命令。

## 5. docs `+update` 成功但 `result: failed`(没编辑权限)

**现象**:`ok: true`,但响应里:
```json
{
  "result": "failed",
  "warnings": ["degrade_code=4030004,msg=Document operation failed: No permission to operate on this document"]
}
```

**根因**:当前账号只有查看权限,没编辑权限。

**注意**:不能只看顶层 `ok` 字段,必须看 `data.result`。`ok: true` 仅代表 HTTP/RPC 调用本身没报错。

**修法**:文档 owner 给当前账号加 `可编辑` 权限,或换一份有编辑权限的目标文档。
