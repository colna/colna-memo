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

## 相关

- [image-gen 参考图怎么选、prompt 怎么配套](image-gen-reference-image.md)
- [AIGC-Studio 生图接口排错](aigc-studio-image-gen.md)
- [浏览器直连 LLM API 的 CORS](browser-direct-llm-api-cors.md)
