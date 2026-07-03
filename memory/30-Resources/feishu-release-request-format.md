---
title: 飞书上线申请规范
date: 2026-06-29
tags: [飞书, 上线申请, 规范, lark-cli, sitin-next]
---

# 飞书上线申请规范

向飞书群发送「上线申请」的统一格式与操作约定(2026-06-29 确立)。

## 发送约定

- **身份**:统一用 **张峥 user**(`lark-cli im +messages-send --as user`)。bot 不在这些群,无法 `--as bot` 代发(会报 `230002 Bot/User can NOT be out of the chat`)。
- **发送方式**:必须用 **`--markdown`**(不是 `--text`),否则加粗不渲染。
- **流程**:拟好文案 → **先发给用户过目** → 用户说「发」再发。
- **常用群 chat_id**:
  - 「前端」`oc_3aabdcfa9738eec37152a5a65dcca0c5`
  - 「前端小分队」`oc_c46f0c8b2ee2b6db95f07080ee7e010e`
  - 「SP后端内部群」`oc_713670054e6236e6074902693bec3595`(SP/social-proxy 后端上线申请发这里,@沈硕;沈硕 open_id 需在当前发送 app `cli_a95cafcf9cb9dcd4` 下重新解析后再补)
  - 发前用 `lark-cli im +chat-search --query 前端 --as user` 确认(同名前缀群很多)。

## 模板格式(富文本加粗版)

字段标签加粗,变更说明独立成段,标签与值之间用全角空格分隔。

```markdown
**【上线申请】<一句话标题>**

**上线项目**　<app-minerva-web、app-minerva-server 等,逗号分隔>
**PR**　<PR 链接>

**变更说明**
<根因 + 改动简述,偏好「稍详细」,可多行>

**测试情况**　<自测:tsc/eslint/circular 等 / 已过测试>
**Code Review**　<本地已 review / 已 CR>
**依赖后端**　<是/否>
**改动数据库**　<是/否,是则简述改了什么>
```

## 字段含义

| 字段 | 填法 |
|---|---|
| 上线项目 | 部署服务名(`app-minerva-web` / `app-minerva-server` 等),多个逗号分隔 |
| PR | PR 链接 |
| 变更说明 | 根因 + 改动,用户偏好稍详细;能一句讲清影响面更好 |
| 测试情况 | 自测写明跑了什么(`tsc.koa` 0 error / eslint / circular);或「已过测试」 |
| Code Review | 本地已 review / 已 CR |
| 依赖后端 | 是/否 |
| 改动数据库 | 是/否;是则简述(如「加了一行手动填充数据」) |

## 已发实例

- PR [#484](https://github.com/presence-io/sitin-next/pull/484)(修 minerva-server prod 构建失败)上线申请,2026-06-29 发到「前端」群。详见 [sitin-next-ci-turbo-cache](./sitin-next-ci-turbo-cache.md) 里的 prod 构建 FAILURE 排错。
- PR [#471](https://github.com/presence-io/sitin-next/pull/471)(social-proxy 控制台 3 块改:devices 修复 + actions V3 重构 + online-stats 5 项统计修复)上线申请,2026-06-30 14:53 发到「前端」群,文末 @尚斌。
- PR [#522](https://github.com/presence-io/sitin-next/pull/522)(minerva 反代对非 JSON 上游响应改流式 pipe,修 SocialProxy CSV 导出 502)上线申请,2026-07-03 01:53 发到「前端」群,文末 @尚斌。首次用张峥默认 app `cli_a95cafcf9cb9dcd4` 直发成功(见下「关键应用配置」更正)。

## 关键应用配置(2026-07-03 更正,以此为准)

- **直接用张峥的默认 app `cli_a95cafcf9cb9dcd4` 发即可** —— 2026-07-03 实测该 app 已授予 `im:message.send_as_user` scope(`lark-cli auth status` 可见),`lark-cli im +messages-send --as user --markdown` 发上线申请成功(message_id `om_x100b6b4188c19c88c251acf9be9d62b`,发到「前端」群)。**不需要**再切别的 profile。
- **旧的 unrestricted app `cli_a948f5747e3b9ccc`(profile `colna-unrestricted`)已失效**:该 profile 已从本地 `~/.lark-cli/config.json` 删除,app secret 也不在本地,**无法重建**(`profile add` 需 `--app-secret-stdin`,没 secret)。别再按旧记忆去 `profile use colna-unrestricted`,会报 `profile not found`。本地现存 profile 只有 `cli_a95cafcf9cb9dcd4`(张峥)和 `yufeifan`(郁斐凡)。
- **踩坑历史**:更早还有个默认 app `cli_a96365d9983e5bb5` 没启 send_as_user,发会 230027 / missing_scope。当前张峥绑的是 `cli_a95cafcf9cb9dcd4`,和它不是同一个,别混。
- 若哪天张峥 token 变 `needs_refresh`:`lark-cli auth login --no-wait --json --domain im` 拿 device_code + verification_url → `lark-cli auth qrcode -o qr.png "<url>"` 生成二维码发用户扫 → 用户确认后 `lark-cli auth login --device-code <code>` 完成。

## open_id 速查(均为 `cli_a95cafcf9cb9dcd4` app 下,per-app,换 app 要重查)

- 张峥:`ou_9607323e76870d4bfed98efd5736f60d`
- 尚斌:`ou_406e58e5a126dac0ca9332a8aab1cf4c`(旧记忆里 `ou_a205...` 是 `cli_a948` app 下的,已作废)
- 解析他人 open_id 最稳的办法:`lark-cli im chat.members get --chat-id <目标群> --as user` 从群成员里按名字捞(@人本就要求是群成员)。

## 发送命令示例

```bash
# 无需切 profile,当前 active 就是张峥默认 app;@人用 <at id="ou_xxx"></at>
MSG="$(cat release-msg.md)"
lark-cli im +messages-send --as user \
  --chat-id "oc_3aabdcfa9738eec37152a5a65dcca0c5" \
  --markdown "$MSG"
```

@人示例:文末加 `<at id="ou_406e58e5a126dac0ca9332a8aab1cf4c"></at>` 即 @尚斌。
