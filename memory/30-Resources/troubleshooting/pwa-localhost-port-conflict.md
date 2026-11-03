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

## 变体（2026-11-03）：crossnote 占 127.0.0.1:3000，`adb reverse` 同端口映射打到它

- **现场**：Vite 让出 3000 起在 3001；`adb reverse tcp:3000 tcp:3000` 后设备访问 `localhost:3000` 命中的是 VS Code crossnote（占 IPv4 `127.0.0.1:3000`），不是 Vite（只监听 IPv6 `*:3000`）。
- **关键点**：`adb reverse` 的**两端端口可以不同**，而设备侧的 URL 由 PWA 自己决定（`LocalTestConf.pwaHost` debug 默认 `http://localhost:3000`，通常不想改代码）。
- **修法**：把设备 3000 反向映射到宿主的 3001，设备 URL 保持 `localhost:3000`：

  ```bash
  pnpm --filter @heyhru/app-pwa dev -- --port 3001
  adb reverse tcp:3000 tcp:3001
  ```

- **排查顺序**：`lsof -nP -iTCP:3000 -sTCP:LISTEN` 看 IPv4/IPv6 归属 → `curl 127.0.0.1:3000` 与 `curl [::1]:3000` 分别对号 → 再决定「改 Vite 端口 + reverse 换宿主端」还是「改宿主 pwaHost」。
- 另一处坑：`android:usesCleartextTraffic="true"`（targetSdk 36 起）不加，WebView 加载 `http://localhost` 直接被拦。
