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

### 网页报 20001,但 CLI 实际已授权成功(2026-07-31)

**现象**:用户点击授权后,浏览器最终页显示「请求不合法,错误码 20001」;但后台轮询日志已经出现 `device-flow: token response received` 和 `OK: 授权成功`。

**判断方法**:不要只看浏览器最终页。先执行 `lark-cli auth list`,再用 `lark-cli auth status --verify` 服务端校验;只要 `tokenStatus: valid`、`verified: true` 且用户名正确,授权就已完成,无需重新扫码。

**本次结果**:本机 profile `cli_a929bdd9c578dcba` 已登录张峥;IM 基础 scopes 有效。`im:message.send_as_user` 仍缺失,上线申请前需单独确认应用后台权限并做增量授权。

### 同一 app 的 user token 不是多账号并存(2026-07-30)

同一 app 下本地只保留一个 user token;给另一账号执行 `auth login` 会覆盖原账号,不是新增一套并存凭据。因此读写仅授权给张峥的文档前,必须先用 `lark-cli auth list` 核对当前账号;不对就重新走张峥的 device flow。不同 app / profile 还要分别核对,因为 user token 和 `open_id` 都具有 app 作用域。

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

## 4.1 `docs +fetch` 读不出内嵌 sheet / 整篇是 sheet 的文档（2026-07-20）

**现象**：文档里的表格章节 fetch 回来只有一个空标签，拿不到单元格内容：

```
<sheet token="I2m9sfnIShfKY8twVgGczSUfnre" sheet-id="ao1JQm"></sheet>
```

若整篇文档本身是电子表格，直接报错：

```
Unsupported document type 'sheet'. Only docx is supported.
```

**影响**：审需求文档时会**静默漏掉整块内容**而不自知 —— 我 review sitin4.0 时「Go Live 页 / Mock 视频改造 / 任务拆解」三节就是这么漏的。

**修法**：从 `<sheet>` 标签取 `token` + `sheet-id`，切到 `lark-sheets` 技能单独读。
**纪律**：fetch 回正文后先 `grep '<sheet\|<bitable'`，有就说明还有下钻内容没读到，别当作已读全。

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

## 6. 知识库 md 导入飞书前必做的三处转换(2026-07-24)

把 `colna-memo` 的笔记原样丢给 `docs +update --doc-format markdown` 会有三类内容坏掉。写入前先跑一遍转换:

| # | 源写法 | 飞书表现 | 转换 |
|---|---|---|---|
| 1 | YAML frontmatter(`---\ntitle: ...\n---`) | 变成一条分隔线 + 一段裸文本 | 整段删掉 |
| 2 | `[[wikilink]]` | `[` 未转义,渲染成半个链接语法,且飞书不认 wikilink | 改成普通描述文字(如「详见知识库笔记「XXX」」) |
| 3 | 正文里裸的 `<` (如 `**仅 APK <1.34 老路径**`) | **被当成 XML 标签起始吞掉后面的内容** | 用反引号包住(`` `<1.34` ``)或转义成 `\<` |

**为什么第 3 条最坑**:`--doc-format markdown` 下 XML 标签**照样会被解析**(`<b>`、`<img>` 都生效),所以裸尖括号不是"显示成 &lt;"而是**静默吃掉一段文本**。反引号内的 `<` 安全,不用管。

**批量自查**:剥掉代码块和行内代码后 grep 裸的 `<` / `[` / `*` / `_`,只处理剩下的少数几处即可 —— 技术笔记里绝大多数特殊字符本来就在反引号里。

**写入指令选择**:目标文档已有内容时用 `append`(不动文档标题);只有确需重建整篇才用 `overwrite` —— 它会连文档标题一起重设,还可能丢图片和评论。
