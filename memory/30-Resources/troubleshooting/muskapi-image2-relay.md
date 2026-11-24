---
title: muskapi 中转 image2（gpt-image-2）可用性与报错口径
date: 2026-09-14
tags: troubleshooting, image-generation, muskapi, gpt-image-2, relay
---

# muskapi 中转 gpt-image-2 报错口径

`https://api.muskapi.cc/v1`（OpenAI 兼容中转）出图时按报错分流，别一上来改代码。
2026-09-14 用新 key 实测（skill：`~/.agents/skills/image2-gen/`）：

| 报错 | 含义 | 怎么办 |
| --- | --- | --- |
| `503 No available compatible accounts` | 模型在该 key 分组里可用，但上游账号池暂时没号 | **等几分钟重试**；generations / edits 会一起波动 |
| `400 当前模型在所选系统分组中不可用` | 本次路由到的分组没有此模型 | 配合 `quality=high` 重试；或换模型 |
| `400 model ... image pricing is missing or invalid for 1K/medium` | 计费表没配这个「尺寸/质量」档位 | 换档位（如 high）；`-4k` / `-adobe` 需要对应尺寸 |

> `-4k` 实测（2026-09-16）：`gpt-image-2-4k` + `1024x1536` 在 **medium 与 high 都被 400**
> （`pricing is missing or invalid for 2K/medium` / `2K/high`）—— 它要的是 4K 档尺寸，
> 而 `gen_image.py` 的尺寸表里没有 → 想靠换 4K 模型提清晰度这条路目前走不通，用默认
> `gpt-image-2` 出图即可（1024 宽在 3x 屏上是 1.7× 放大，软景别能接受）。
| `404 Request for model ID ... not found` | 模型不走这个端点 | Gemini 图像模型走 `/chat/completions`，不是 `/images/*` |
| `403 Chat Completions API is not enabled for this group` | key 分组没开 chat 端点 | gpt-image-2 走 `/images/generations` 或 `/images/edits` |
| `403 GROUP_NOT_ALLOWED` +「根据您所在区域的法律条款…」 | **按出口 IP 的地域拦截**，不是 key / 模型 / 分组的问题 | 开代理即可（见下） |

## 403 GROUP_NOT_ALLOWED（地域拦截，2026-09-15 实测）

判断口径：**首页 `GET https://api.muskapi.cc/` 返回 200，但 `/v1/models` 与 `/v1/images/*` 全 403** ——
说明站点可达、拦截只落在 API 上，换模型（`gpt-image-2.5` 等）同样 403，别在 key 上找原因。

修法（本机）：`open -a "Clash Verge"`。本机 Clash Verge 配置里
`enable_system_proxy: true` + `enable_tun_mode: true`，**启动即自动开系统代理**（mixed-port 7897），
无需再点「系统代理」开关。判断已生效：

```sh
scutil --proxy | grep -E "HTTPEnable|HTTPPort"   # HTTPEnable : 1 / HTTPPort : 7897
curl -s -o /dev/null -w "%{http_code}" https://api.muskapi.cc/v1/models -H "Authorization: Bearer $KEY"  # 200
```

Python `urllib`（gen_image.py 走的库）默认读 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量；
但本机 TUN 模式会直接接管路由，**不开代理时 curl 也能通**，两种路径都实测过。
出完图若用户不想要代理，记得别自作主张关掉它 —— 那是用户自己的网络设置。

## 端点分工（实测）

- `gpt-image-2` → `/images/generations`（文生图）/ `/images/edits`（图生图，1 张字段 `image`、多张 `image[]`）。
- Gemini 系图像模型（`gemini-3.1-flash-image` 等）→ `/chat/completions`，回复里是 `data:image/jpeg;base64,...`，**不是** `/images/*`。
- `quality` 显式传 `high` 才过计费校验（默认 medium 档位可能没配价）。

## 耗时

- 图生图单张：72s（09-09 旧 key）／ **261s**（09-14 新 key，上游拥挤时）。
- 客户端超时别低于 300s；慢 ≠ 卡死，别提前杀。

## 不要并发（2026-09-22 实测）

三条 `--ref` 图生图并行发（同一 key、同一时刻）：1 条 45s 成功，另两条在
`h.getresponse()` 处 `http.client.RemoteDisconnected: Remote end closed connection without response`
—— 网关侧直接断连，不是 5xx 回包。**重试串行即可恢复**（本题 A/B 两张随后一次串行跑完，
183s / 277s）。多版出图的正确姿势：写多个 `--prompt-file`，**一条命令里 `&&` 串行**，
不要用 `&` 并发；失败那两条自动重试时也不用改 prompt（不是 prompt 的问题）。

## gpt-image-2.6 可用性（2026-11-12 实测）

- `/v1/models` 已列出 `gpt-image-2.6` / `-sunburst` / `-flare`（created 2026-11-05），但目录列表 ≠ 账号分组可用。
- 本机 key 实测：
  - `gpt-image-2.6` → 502 `system service temporarily unavailable`，重试后 400 `当前模型在所选系统分组中不可用`；加 `quality=high` 仍是 502。
  - `gpt-image-2.6-sunburst` / `-flare` → 连续 502 `system request failed`。
  - 同 key 的 `gpt-image-2` / `gpt-image-2.5-sunburst` → 400 分组不可用。
- 结论：当前 key 分组未启用 2.6（甚至没有可用图像模型），或上游 2.6 不稳定；**image2.6 暂时不能出图**。先找管理员把 2.6 加入分组，或改用 AIGC 2.5。

## 相关

- [image-gen 参考图怎么选、prompt 怎么配套](image-gen-reference-image.md)
- [AIGC-Studio 生图接口排错](aigc-studio-image-gen.md)
- [浏览器直连 LLM API 的 CORS](browser-direct-llm-api-cors.md)

## image2.6 GIF 输出：官方参数 vs 本机中转实测（2026-11-24）

**官方口径**（OpenAI `developers.openai.com/api/docs/guides/image-generation` + API reference）：

| 字段 | 值 | 默认 |
| --- | --- | --- |
| `model` | `gpt-image-2.6-sunburst` / `gpt-image-2.6-flare`（2.6 没有日期快照） | — |
| `output_format` | `png` / `jpeg` / `webp` / `gif` / `svg` | `png` |
| `frame_count` | 2–16（整数），仅 gif 生效 | 9 |
| `frame_duration_ms` | 20–2000，四舍五入到 10ms，编码上限 655350ms | 200 |
| `size` | `auto` 或 `WIDTHxHEIGHT`（两侧 16 的倍数，描述单帧）；边长 ≤3840、宽高比 1:3~3:1、总像素 655,360–8,294,400 | `auto` |
| `n` | 1–10，帧数×尺寸×n 要一起过像素预算 | 1 |

- 端点是 `/images/generations`；guide 也写了 `/images/edits` 可出 GIF，但 API reference 注明 GIF 不支持 mask。
- GIF 不支持：streaming、`partial_images`、`output_compression`、`prompt_rewriter`；`background: transparent` 只能配 png/webp。
- 有账号级门槛：reference 写「Requires access to the GIF 预览 for the selected model」。
- 回复里 `data[].b64_json` 解码后按 `.gif` 存；`quality` 对 2.6 支持 `low/medium/high/xhigh/max`，另有 `complexity: low/medium/high`（2.6 新增，控制规划开销）。
- 计费与 2.5 同价：$8/1M 图像输入 token、$30/1M 输出 token；2.6 不进 Batch。
- 注意博客口径混乱：有文章写 `frame_count` 上限 64、或 `frame_size` 单独字段——官方当前文档是 2–16 且只有 `size`，不要照抄。

**本机 muskapi 中转实测（11-24）**：

```
POST /v1/images/generations  model=gpt-image-2.6-sunburst  output_format=gif  frame_count=4  frame_duration_ms=200  size=512x512
→ HTTP 200, 175s, model 回显 gpt-image-2.6-sunburst
→ 顶层 output_format: "png"，data[0].format: "png"，fallback_reason: "upstream_output_mismatch"
→ 落盘是标准 PNG（无 acTL，不是 APNG）
```

结论：中转**接受** gif 参数但不透传 GIF 能力，静默降级成 PNG 并给 `fallback_reason`。判断方法：请求后必须读顶层 `output_format`（或 `data[].format`），不能只看 HTTP 200。要真 GIF 只能走官方 OpenAI key（本机未配 `OPENAI_API_KEY`）或有 GIF 权限的中转。

## 各图像模型可用性实测（2026-11-24，512×512 探针）

同一句 prompt、`n=1`、无 quality 参数、串行请求，结果：

| 模型 | HTTP | 耗时 | 结果 |
| --- | --- | --- | --- |
| `gpt-image-2` | 200 | 28.7s | 出图（旧版响应，无 output_format/size 字段） |
| `gpt-image-2.5-sunburst` | 200（首次 300s 客户端超时） | 20.7s（重试） | 出图 PNG 185KB；不是坏，是排队慢 |
| `gpt-image-2.5-flare` | 200 | 118.6s | 出图 |
| `gpt-image-2.6` | 200 | 51.3s | 出图 |
| `gpt-image-2.6-sunburst` | 200 | 12.8s | 出图 PNG 244KB，最快 |
| `gpt-image-2.6-flare` | 502 → 503 | 70.3s / 1.0s | `system service temporarily unavailable` → `No available compatible accounts`，上游无号 |

- 结论：11-12 记的「分组未开通」已恢复，2/2.5/2.6 的 sunburst 与 `gpt-image-2.6` 可用；**2.6-flare 无可用上游账号**。
- 客户端超时至少给 600s，300s 会把还在排队的 2.5-sunburst 误判为失败。
- 成本观测：7 次请求（含 1 次 GIF 探测、1 次超时）共扣 **$0.84**（6.6892 → 5.8492），512×512 单张约 $0.1~0.12。
- 余额查询：`GET /v1/dashboard/billing/subscription` → `hard_limit_usd` 即剩余额度；`/dashboard/billing/usage` 恒为 0，不可用。
- 站点：主站 `muskapi.cc`（MuskAI），控制台 `elonai.cc`，文档 `docs.muskapi.cc`，API 页标题 `ElonAI - AI API Gateway`。
