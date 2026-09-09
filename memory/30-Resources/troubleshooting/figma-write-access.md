---
title: Figma 写入能力 —— REST API 写不了画布,只能靠 Plugin
date: 2026-09-09
tags: [troubleshooting, figma, api, plugin]
---

# Figma 写入能力:REST API 写不了画布

## 坑

以为「token 有 write 权限」或「我对文件有编辑权」就能让脚本/agent 往 Figma 稿子里加图层。**不能。**

## 根因

Figma REST API **根本没有创建/修改节点的 endpoint**。全部 write scope 只有:

| scope | 能写什么 |
|---|---|
| `file_comments:write` | 稿子上发评论 |
| `file_dev_resources:write` | 给节点挂 dev 链接 |
| `file_variables:write` | Variables/设计变量(**仅 Enterprise**) |
| `webhooks:write` | Webhook |

画布内容(Frame/Text/组件…)只有 **Figma Plugin API**(`figma.createFrame()` 等)能改,插件跑在用户的 Figma 客户端内、以用户身份操作 —— 这时文件编辑权限才是必要条件。

官方 MCP(`mcp__figma__get_figma_data` / `download_figma_images`)也是**只读**。

## 修法

写一个本地插件 + 本地 HTTP 桥,让命令行驱动画布写入。实现在
`/Users/max/Dev2/zhangzheng/figma-write-bridge/`(零依赖 Node):

- `bridge.mjs` — 127.0.0.1:3055;`POST /exec`(CLI 投命令,阻塞等结果)、`GET /poll`(插件长轮询 25s)、`POST /result`
- `plugin/{manifest.json,ui.html,code.js}` — 插件
- `figma.mjs` — CLI:`node figma.mjs <cmd> '<argsJson>|@file.json'`
- 命令:`ping` / `get_selection` / `get_page` / `create` / `update` / `clone` / `move` / `delete`

### 实现要点(踩过的)

1. **插件主线程是 QuickJS 沙箱,没有 `fetch` / `WebSocket`**。网络只能在 `ui.html` 这个 iframe 里做,再 `postMessage` 转给主线程。
2. `manifest.json` 必须声明 `networkAccess.allowedDomains: ["http://127.0.0.1:3055"]`,否则 iframe 的 fetch 被拦。
3. **导入本地插件只能用 Figma 桌面版**:Plugins → Development → Import plugin from manifest…;网页版没这个入口。
4. 插件窗口关掉就断连,长任务要让窗口一直开着。
5. `TEXT` 节点设 `characters` 前必须 `await figma.loadFontAsync(fontName)` —— 建树前先遍历 spec 收集所有字体一次性加载。
6. 插件改动**直接落真实文档**(可 Cmd+Z / 版本历史恢复)。动他人现有图层前要先确认。
