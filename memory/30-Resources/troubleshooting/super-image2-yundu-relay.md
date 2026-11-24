---
title: super-image2 接 gd.yundu.lol 图像中转（CORS / 参数 / 尺寸）
date: 2026-11-24
tags: troubleshooting, super-image2, image-generation, cors, relay, yundu
---

# gd.yundu.lol 中转要点

Base URL（super-image2 前端直接拼接 `/images/generations`、`/models`，必须带 `/v1`）：
`https://gd.yundu.lol/v1`。`x-openai-actor-authorization: local-image-extension` 不加也能通。

## 浏览器直连必失败（CORS）

```
OPTIONS /v1/models  Origin: http://localhost:3000  → 403，无 Access-Control-Allow-Origin
GET     /v1/models  Origin: ...                    → 200，但仍无 ACAO
```

「测试连接」显示连接失败不是 key 错，是预检被拒。修法：所有请求走同源
Next API route（`/api/test-connection`、`/api/generate`、`/api/edit`），服务端转发无 CORS。
判断口径同 `browser-direct-llm-api-cors.md`：DevTools 里「已传输 0 B + 预配标头」= 请求没发出去。

## size 不校验，探活会烧额度

传 `size: "999x999"` 不报参数错误，**照样真出图**。想低成本探测模型是否可用别用非法参数，
直接看 `/v1/models` 或接受一次真实出图成本。

## 实测

| 项 | 结果 |
| --- | --- |
| `/v1/models` | 12 个，其中图像 5 个：`gpt-image-2.6` / `-2.5-sunburst` / `-2.5-flare` / `-2.5` / `-2` |
| 出图耗时 | 1024×1024、quality low，约 66s（sunburst 更慢） |
| 响应 | `data[].b64_json`；`gpt-image-2.5-flare` 会回 `revised_prompt` |
| 高级字段 | `output_format` / `complexity` / `moderation` / `background` 不报错，能透传 |
| 超时 | 客户端别低于 300s，45s 探活会截断响应 |

## super-image2 改造（2026-11-24）

- 服务商配置只留连接信息，模型在输入区下拉选（`availableModels` 由测试连接拉取）。
- 生成/改图从前端直连改为 `/api/generate`、`/api/edit`，顺带修了 CORS。
- 「更多」参数：复杂度 / 格式 / 帧数 / 每帧时长 / 透明背景 / 审核；帧数两项仅 GIF 可编辑。
- 坑：antd `Radio` 只写 `onClick stopPropagation` 时点圆圈不会切换服务商，需要 `onChange`。
