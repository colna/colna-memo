---
title: metabot Codex 模型名非法与 reasoning effort 设置
date: 2026-09-20
tags: [troubleshooting, metabot, codex, deepseek]
---

# metabot Codex 模型名非法与 reasoning effort 设置

## 坑 1：`/model <name>` 报 invalid_request_error

现象：飞书里发 `/model deepseek-v4.1-flash` 后，机器人回「⏳ Codex notification — The supported API model names are deepseek-flash, deepseek-v4-pro, but you passed deepseek-v4.1-flash.」

根因：`~/.codex/config.toml` 的 provider 指向 `https://api.deepseek.com/v1`（网关，非官方模型名），该网关 `/v1/models` 只提供两个模型。`/model` 只是把名字透传给 `codex exec -m <name>`（见 `src/engines/codex/executor.ts` 的 `buildCodexArgs`），名字不存在就直接被上游拒绝，不是桥接层 bug。

核查命令（不打印 key）：

```bash
KEY=$(jq -r .OPENAI_API_KEY ~/.codex/auth.json)
curl -s -H "Authorization: Bearer $KEY" https://api.deepseek.com/v1/models
# => deepseek-flash, deepseek-v4-pro
```

修法：`/model deepseek-v4-pro`、`/model deepseek-flash`，或 `/model reset` 回落到 config 默认。

## 坑 2：effort（思考强度）在聊天里设不了

现状：metabot 的 Feishu 命令只封装了 `/model`，没有 `/effort`。effort 真源是 Codex 配置 `model_reasoning_effort`，本机当前为 `ultra`。

合法取值（从 codex 二进制里确认）：`minimal` / `low` / `medium` / `high` / `xhigh` / `max` / `ultra`；另有 `plan_mode_reasoning_effort` 控制 Plan 模式。

两种改法：

1. 全局：改 `~/.codex/config.toml` 的 `model_reasoning_effort`（同时影响终端 CLI）。
2. 只改 metabot：`bots.json` 的 `codex.extraArgs` 追加 `["-c", "model_reasoning_effort=\"high\""]`（`extraArgs` 会被塞进 `codex exec` 前的全局参数位，`-c` 可用；值必须带引号，TOML 解析），改完重启 metabot（pm2）。
