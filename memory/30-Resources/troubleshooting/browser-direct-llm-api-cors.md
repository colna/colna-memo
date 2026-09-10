---
title: 纯前端应用直连 AI 中转站报 Failed to fetch(CORS 白名单)
date: 2026-09-10
tags: troubleshooting, cors, super-image2, llm-gateway, browser
---

# 纯前端直连 AI 中转站:Failed to fetch

## 现象(2026-09-10 · SuperImage 配 muskapi)

`https://super-image2.vercel.app` 里加服务商 `musk`(Base URL `https://api.muskapi.cc/v1`),
「测试连接」报**连接失败**,页面报 **`Failed to fetch`**。DevTools 网络面板:
请求 `/v1/models` 红叉、**已传输 0 B**、请求头栏提示**「当前显示的是预配标头」**。

## 一眼判据

**「已传输 0 B」+「预配标头(provisional headers)」= 请求压根没发出去**,
不是服务器返回了错误码。这两条同时出现,先查 CORS / 网络层,**别去查 API key 对不对**
——key 错会拿到 401 响应体,不会是 0 B。

## 根因

服务端 curl 完全正常(说明 DNS/TLS/路由/服务都没问题):

```bash
curl https://api.muskapi.cc/v1/models   # → 401 API_KEY_REQUIRED,0.63s
```

但预检被拒,且**响应里没有 `Access-Control-Allow-Origin`**:

```
OPTIONS /v1/models  →  403
access-control-allow-methods / allow-headers / max-age 都有
Access-Control-Allow-Origin                     ← 唯独缺这个
vary: Origin                                    ← 说明按 Origin 分流
```

按 Origin 逐个试,白名单只有它自己:

| Origin | 预检 | ACAO |
|---|---|---|
| `https://super-image2.vercel.app` | 403 | 无 |
| `http://localhost:3000` | 403 | 无 |
| `https://chat.openai.com` | 403 | 无 |
| `https://api.muskapi.cc`(自身) | 204 | ✅ |

**muskapi.cc 只面向服务端调用,不支持任何浏览器跨域。** 而 SuperImage 是纯前端
(key 存 localStorage、浏览器直接 fetch 供应方),所以必然失败。截图里 OpenAI 那个
服务商是绿点,正因为 `api.openai.com` 允许跨域。

## 解法

1. **换一个允许跨域的中转** —— 实测 `callapi.top` 是 `204 + ACAO: *`,
   两个 Origin 都放行,改 Base URL 即可,不用动代码。
2. **加一层同源代理**:Vercel 上加 route handler / serverless function 转发到中转站。
   服务端之间没有 CORS,顺带把 key 从浏览器挪到服务端。
3. 本地开发可临时反代,但线上仍要 1 或 2。

## 复用要点

**验一个 API 能不能被网页直连,只要一条命令**:

```bash
curl -s -i -X OPTIONS '<base>/v1/models' \
  -H 'Origin: https://<你的站点>' \
  -H 'Access-Control-Request-Method: GET' \
  -H 'Access-Control-Request-Headers: authorization' | head -5
```

要 **2xx 且带 `Access-Control-Allow-Origin`**,缺一不可。选中转站前先跑这条,
比部署完再排查省一天。
