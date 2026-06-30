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

- PR [#484](https://github.com/presence-io/sitin-next/pull/484)(修 minerva-server prod 构建失败)上线申请,2026-06-29 发到「前端」群。详见 [[sitin-next-ci-turbo-cache]] 里的 prod 构建 FAILURE 排错。

## 发送命令示例

```bash
# 文案写进变量后用 --markdown 发(--as user = 张峥)
lark-cli im +messages-send --as user \
  --chat-id "oc_3aabdcfa9738eec37152a5a65dcca0c5" \
  --markdown "$MSG"
```
