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

## 2026-09-14 跑通时踩的坑（全部实测）

1. **`networkAccess.allowedDomains` 不能写 IP**。`http://127.0.0.1:3055` 被 Figma 判
   `Manifest error: Invalid value for allowedDomains. … must be a valid URL.`；换成
   `http://localhost:3055` 才通过。**且 `ui.html` 里 fetch 的 host 必须与它逐字一致** ——
   写 `127.0.0.1` 会被沙箱拦掉，桥侧一条请求都看不到（只能看到插件"没反应"）。
2. **Dev Mode（右侧 Inspect 面板）下导入**，manifest 的 `editorType` 必须含 `"dev"`，
   否则报 `The manifest editorType does not include "dev".`。
3. **`figma.vectorPaths` 只吃 `M/L/H/V/C/Z`**：`A/a`（圆弧）与 `S/T/Q`（平滑）都会报
   `Failed to convert path. Invalid command at …`。SVG 图标要先转三次贝塞尔（本仓库
   `lib/patharc.mjs` 做了 A→C、Q→C、S→C、T→C 与相对/绝对归一）。
4. **`strokeCap` 没有 `BUTT`**，合法值里对应的是 `NONE`。
5. **`figma.createFrame()` 等创建函数会立刻把节点挂到当前页**：命令中途报错重试会在页面上
   留一堆游离节点（`Vector` / `Text` / 半个屏）。收尾要 `get_page` 扫一遍顶层、删掉非
   section 的残渣。
6. **`appendChild` 保留子节点相对坐标**：先给子节点写绝对坐标再 append 到父节点，会被叠加
   一次父级偏移（section 在 (894,4700) → 子节点跑到 (1848,9440)）。子节点一律写相对坐标。
7. **桥自身的 bug**：`/exec` 把任务交给长轮询后不能把 job 丢掉，否则 `/result` 匹配不上、
   永远超时。要用 `Map<id, job>` 保留到 `/result` 到达。
8. `loadFontAsync` 要先收集整棵树用到的字体再一次性加载；`figma.loadFontAsync` 对
   "Nunito ExtraBold" 这类 `family + style` 组合可用（Figma 自带 Google Fonts）。

## 2026-09-15 VECTOR 的 bbox / resize 语义（分层复刻时实测）

写 50 · Bond 的圆环（整圈 + 75% 进度弧 + 18° 起点刻度，都靠 `stroke` 画）时探到的：

1. **VECTOR 的节点 bbox = 路径几何边界，不含描边**。实测：24×24 的方 + `strokeWeight 18`，
   创建时不传 `width/height`，`find` 回读是 **24×24**（不是 42×42）；小弧的回读值就等于弧的
   几何外接盒（29.36×4.65）。
2. **插件对 VECTOR 先写 paths/stroke 再 `node.resize(width, height)`，而 resize 会按几何 bbox
   缩放**。所以「声明成整环尺寸、路径只是其中一小段弧」的写法，会把那一小段弧拉到整环大小
   （`strokeWeight * scale` 只补描边粗细，补不回形状）。
3. **修法**：每条描边矢量按**自己的几何 bbox** 声明 —— `width/height` = 该路径的几何外接盒，
   `viewBoxSize` 取同值（`scale = width/viewBoxSize = 1`，描边粗细不被二次缩放），`x/y` = 该
   bbox 相对上层的左上角。描边天然会画出 bbox 之外，不用把描边算进声明尺寸（算进去反而会被
   缩一次）。实测这样建出来的三条矢量 bbox 与手算值逐个吻合。
4. **路径数据里的坐标偏移会被归一化**：同一路径数据平移 0 / +100 / −50，声明 `x: 0` 后三个
   节点的 bbox 都落在 0 —— Figma 以 bbox 定位节点，路径的内部偏移不影响落点（但会影响
   `resize` 的目标 bbox，见第 3 条）。
5. **探法**（可复用）：`create` 时故意不传 `width/height`，节点保留自然 bbox，再用 `find`
   回读；scratch 造完立即 `delete`，不留在画面上。核心脚本在
   `/var/folders/.../opencode/probe-bbox.mjs`（一次性）。

