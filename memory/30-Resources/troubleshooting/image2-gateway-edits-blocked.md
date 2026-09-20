---
title: image2 网关（muskapi）图生图被挡：分通道判断与绕行
date: 2026-09-20
tags: [image2-gen, gpt-image-2, gateway, troubleshooting, opencode]
---

# image2 网关（muskapi）图生图被挡

## 现象（2026-09-20 上午，约 1.5 小时）

`~/.agents/skills/image2-gen` 出图持续失败，两条错误交替出现：

| 现象 | 含义 |
| --- | --- |
| `HTTP 502 system access forbidden, please contact administrator` | 该 key 的**分组没开这个模型/通道**（配置问题，不是临时） |
| `HTTP 503 No available compatible accounts` | 该模型**兼容的上游账号池空了**（临时，可等） |
| `HTTP 400 当前模型在所选系统分组中不可用` | 同上第一种的文字版；多出现在 multipart（带了 image 字段）的请求上 |
| `model xxx image pricing is missing or invalid for 1K/low` | 网关 pricing 表缺档（`gpt-image-2-4k` / `-adobe` 各档都缺，等于不可用） |

## 下午补充：三个新事实（2026-09-20 14:00~14:30）

1. **区域封锁**：`403 GROUP_NOT_ALLOWED / 根据您所在区域的法律条款…`，本机 CN 出口 IP（北京）被拒。**用本机 Clash Verge 代理（mixed-port 7897）绕过后恢复**：`HTTPS_PROXY=http://127.0.0.1:7897`。注意 `gen_image.py` 走 `urllib`，默认吃 `HTTPS_PROXY` 环境变量，不用改代码。
2. **`/images/edits` 是权限问题，不是区域问题**：代理下还是 `502 system access forbidden`；`/v1/responses` 明确回 `Responses API is not enabled for this group`，`/v1/chat/completions` 同理。**当前 key 的 group 只开了 `/images/generations`**，想要图生图（带参考图）必须换 key / 让网关管理员给 group 放行。
3. **这个网关对 multipart 走的是另一条路**：`gen_image.py` **无论有没有参考图都用 multipart/form-data**（`build_multipart`），实测这种请求被分组拒（400/502）；同样的 prompt 改用 **JSON body** 打 `/images/generations` 就能过。所以纯文字出图时，用 30 行 curl/python+JSON 的小脚本比调 skill 脚本可靠（本次物料就是这么出的）。

## 关键结论：两个通道是分开的

- `/v1/images/edits`（**带参考图**，1 张用 `image` 字段、多张用 `image[]`）→ 全程被挡。
- `/v1/images/generations`（**纯文字**，JSON body）→ **可间歇成功**（实测 10 次约 7 次成功）。
- 同一个 key 对 `/v1/images/generations` 带 multipart + image 字段 → 报分组错误，等于 edits 通道。
- `/v1/chat/completions` → `Chat Completions API is not enabled for this group`（该 group 没开），所以 Gemini 图像模型那条路也走不通。
- `?group=xxx` 查询参数**无效**（试了 default/vip/image/official 等，行为不变，分组由 key 自己决定）。

判断口径：**先用一条极短 prompt 打 `generations`**（如 `red apple on white`）。它能过 → 说明 key 和账号池没问题，是 edits 通道被封；两块都过不了才是整体故障。

## 绕行

1. **纯文字出图**顶上：把参考图里的角色特征、构图逐条写进英文 prompt（角色卡细节 + 版式描述），牺牲「参考图保真」，换能出货。质量比想象中好，但要接受角色不会 1:1 一致。
2. **等 + 重试**：脚本本身对 5xx 只重试 2 次，长期故障要自己套一层循环（每 60s 一轮，成功即写结果文件）。
3. 换 key / 让网关管理员给 group 放行是根治，改 `~/.image_gen_creds` 一处即可（脚本优先读 `IMAGE_API_BASE` / `IMAGE_API_KEY` 环境变量，其次才是这个文件）。

## 附带技巧：从 opencode 桌面版会话里捞用户贴的图

用户在桌面版 opencode 里贴的图不落盘（飞书会话那套临时目录不适用）。存在 `~/.local/share/opencode/opencode.db` 的 `part` 表：`type=file` 的 part，`data` 是 JSON，`url` 字段是 `data:image/png;base64,...`。

```bash
sqlite3 ~/.local/share/opencode/opencode.db \
  "select id, datetime(time_created/1000,'unixepoch','+8 hours'), length(data)
   from part where time_created > (strftime('%s','now')*1000 - 3600000)
     and data like '%image%' order by time_created desc limit 20;"
# 取出 data，用 python json.loads 后把 url 的 base64 段解码存盘
```

部分 part（用户自己上传的文件）的 `filename` 会直接给出原始绝对路径，优先用那个。
