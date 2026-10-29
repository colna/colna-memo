# PWA 手机访问 localhost:3000 显示「未知路由」

## 症状

手机通过 `localhost:3000`（或 `adb reverse`）访问本地 PWA dev server，页面返回：

```json
{"error":{"code":"NOT_FOUND","message":"未知路由"}}
```

不是 React 路由 fallback（app 的 `*` 会 redirect 到 `/` 或 `/onboarding`），也不是 Vite 页面。

## 根因

Mac 的 3000 端口同时被两个进程监听：

```text
node .../app-pwa/node_modules/.bin/vite          IPv6 *:3000
node .../social-proxy-agent/src/index.ts         IPv4 127.0.0.1:3000
```

- Vite 监听 IPv6 wildcard，LAN IP（`http://<mac-ip>:3000`）能命中 Vite。
- `social-proxy-agent` 只监听 IPv4 `127.0.0.1:3000`。
- `localhost` / `adb reverse` 默认走 IPv4 127.0.0.1，命中的是 `social-proxy-agent`，它返回 `未知路由`。

## 快速确认

```bash
lsof -nP -iTCP:3000 -sTCP:LISTEN
curl -sS http://127.0.0.1:3000/        # social-proxy-agent → 未知路由
curl -sS http://$(ipconfig getifaddr en0):3000/  # Vite → HTML
```

## 解决

1. 手机直接用 Mac LAN IP（推荐，Vite 已 `host: true`）：

   ```bash
   ipconfig getifaddr en0
   # 手机浏览器打开 http://<mac-ip>:3000
   ```

2. 换一个端口跑 Vite，再用 LAN IP 或 adb reverse：

   ```bash
   pnpm --filter @heyhru/app-pwa exec vite --host --port 3001
   adb reverse tcp:3001 tcp:3001
   # 手机打开 http://localhost:3001
   ```

3. 如果 `social-proxy-agent` 不需要，停掉它后再用 3000（先确认该进程用途）：

   ```bash
   kill 77503
   ```

## 备注

- 手机上的 `localhost` 默认是手机自己；不用 `adb reverse` 时，应该用 Mac 的 LAN IP。
- 这次不是 service worker 缓存，也不是前端路由问题；响应体是 `social-proxy-agent` 的 404 JSON。
