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
| `404 Request for model ID ... not found` | 模型不走这个端点 | Gemini 图像模型走 `/chat/completions`，不是 `/images/*` |
| `403 Chat Completions API is not enabled for this group` | key 分组没开 chat 端点 | gpt-image-2 走 `/images/generations` 或 `/images/edits` |

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
