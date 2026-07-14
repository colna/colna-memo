---
title: 给本地 SPA / 纯 UI 组件做 headless 截图(Chrome + CDP over ws）
date: 2026-07-13
tags: [troubleshooting, screenshot, headless, chrome, cdp, vite, spa, frontend]
---

# 给本地 SPA / 纯 UI 组件做 headless 截图

场景:MetaBot 里要「把某个页面/组件样式截图发飞书」。没有 chrome-devtools MCP、没装 puppeteer/playwright 时的可靠做法。来源:sitin-next app-pwa 语音 pill ×3 徽章(2026-07-13,PR #600)。

## 结论(先看这个)

- **纯 UI 组件**(样式无关登录态/后端)→ **首选方案 B:静态 HTML 逐行复刻 markup**,不碰 dev server,最省事最可控。
- **必须跑真实页面**(依赖真实数据/交互)→ **方案 A:Chrome headless + CDP over `ws`**,轮询等目标元素出现再截。
- **不要**用 `chrome --headless --screenshot=... URL`(即使加 `--virtual-time-budget`):它在 load 事件就截,等不到 React mount / Vite 瀑布 ESM 加载完 → 截到 `index.html` 的 `#skeleton-loader` 骨架。

## 方案 A:Chrome headless + CDP(等元素再截)

系统 Chrome 已有(`/Applications/Google Chrome.app/...`),`ws` 一般在前端 monorepo 的 node_modules 里。脚本放 scratchpad 时用**绝对路径** import `ws`(否则 `ERR_MODULE_NOT_FOUND`)。

```js
// shot.mjs — node shot.mjs "<url>" "<out.png>"
import { spawn } from "node:child_process";
import { writeFileSync } from "node:fs";
import WebSocket from "/绝对路径/项目/node_modules/ws/index.js"; // scratchpad 脚本必须绝对路径

const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const URL = process.argv[2], OUT = process.argv[3], PORT = 9333;
const WAIT_TEXT = "Voice reply"; // ← 改成目标页面上必然出现的一段文字

const chrome = spawn(CHROME, [
  "--headless=new", `--remote-debugging-port=${PORT}`, "--remote-allow-origins=*",
  "--hide-scrollbars", "--force-device-scale-factor=2", "--window-size=430,880",
  "--no-first-run", "--no-default-browser-check", URL,
]);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// 拿 page target 的 webSocketDebuggerUrl(直接连 page ws,免 session flatten）
let wsUrl;
for (let i = 0; i < 40 && !wsUrl; i++) {
  try {
    const list = await (await fetch(`http://localhost:${PORT}/json`)).json();
    wsUrl = list.find((t) => t.type === "page" && t.webSocketDebuggerUrl)?.webSocketDebuggerUrl;
  } catch {}
  if (!wsUrl) await sleep(250);
}
const ws = new WebSocket(wsUrl, { perMessageDeflate: false });
let id = 0; const pending = new Map(); const errors = [];
ws.on("message", (b) => {
  const m = JSON.parse(b.toString());
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); }
  if (m.method === "Runtime.exceptionThrown")
    errors.push("EXC: " + (m.params.exceptionDetails?.exception?.description || "").slice(0, 300));
});
const send = (method, params = {}) =>
  new Promise((res) => { const mid = ++id; pending.set(mid, res); ws.send(JSON.stringify({ id: mid, method, params })); });

await new Promise((r) => ws.on("open", r));
await send("Page.enable"); await send("Runtime.enable");
await send("Page.navigate", { url: URL });

let ok = false;
for (let i = 0; i < 60; i++) { // ~24s
  const r = await send("Runtime.evaluate", {
    expression: `document.body && document.body.innerText.includes(${JSON.stringify(WAIT_TEXT)})`,
    returnByValue: true,
  });
  if (r?.result?.value === true) { ok = true; break; }
  await sleep(400);
}
await sleep(600); // 字体/布局稳定
const cap = await send("Page.captureScreenshot", { format: "png" });
writeFileSync(OUT, Buffer.from(cap.data, "base64"));
console.log(ok ? "captured" : "TIMEOUT — 元素没出现"); console.log(errors.join("\n"));
ws.close(); chrome.kill(); process.exit(0);
```

**诊断(截到 skeleton / 空白时)**:CDP `Runtime.evaluate` dump `document.getElementById('root')?.innerHTML` +
监听 `Runtime.exceptionThrown` / `Runtime.consoleAPICalled`。`rootKids>0` 但 `innerText:""` = React mount 了但被
路由 Suspense fallback / gate 卡住;停在 `#skeleton-loader` = React 根本没 mount(看 exception)。

## 方案 B:静态 HTML 逐行复刻(纯 UI 组件)

照组件的 JSX 把 markup + 样式抄成一个自包含 HTML(Tailwind class 翻成等价 CSS,颜色/尺寸/SVG 图标 1:1),
`file://` 直接截图,不依赖 dev server / React / 登录态:

```bash
"$CHROME" --headless=new --disable-gpu --hide-scrollbars \
  --force-device-scale-factor=3 --window-size=460,150 \
  --default-background-color=00000000 \
  --screenshot="out.png" "file:///abs/pill.html"
```

方案 B 用 `--screenshot` 没问题(静态页 load 即完成,不需要等 React)。视觉与真实组件一致的前提是**逐行照 JSX 抄**,并在图上标注这是复刻预览、附真实源码位置。

## 关键踩坑

1. **`--virtual-time-budget` 对 Vite dev 不可靠**:Vite dev 的瀑布式 ESM import + React 异步 mount,virtual time 到期常仍停在 `index.html` 骨架。要「等元素」必须走 CDP 轮询(方案 A)。
2. **改 monorepo 里被 Vite 预构建的包会污染 dev**:sitin-next 里 `pnpm --filter business-pwa-proto build`(重建 proto gen)改动了 root lockfile → Vite `Re-optimizing dependencies because lockfile has changed` → react `jsx-dev-runtime` 实例不一致 → 页面崩 `TypeError: _jsxDEV is not a function`,**清 `node_modules/.vite` + 重启也没恢复**。→ 纯 UI 优先方案 B,别为截图去动 dev 依赖状态。
3. **`ws` import 路径**:scratchpad 里的脚本 resolve 不到项目 node_modules,用绝对路径 import。项目根跑 node 时脚本要放项目内(node 从脚本所在目录向上找 node_modules,不看 cwd)。
4. **发飞书**:截好的图 `cp` 到环境变量 `OUTPUTS_DIR` 指向的目录(MetaBot 自动扫描发回)。
5. Chrome flags:`--headless=new`(新版 headless)、`--force-device-scale-factor=2/3`(retina 清晰)、`--window-size=宽,高`(移动端用 430×880)、`--remote-allow-origins=*`(CDP 连接必须,否则 403)。

相关:iOS `<video>` 黑屏截图定位见 [[pwa-mobile-gesture-media]];app-pwa dev 登录态验证走不通见该文「验证环境坑」。
