---
title: IG 脚本真机测试链路（小崽 WebView + WS Debug + 本地浏览器）
date: 2026-10-27
tags: [instagram, app-ins-scripts, social-proxy, android, webview, testing]
---

# IG 脚本真机测试链路

从 `sitin-next/packages/app-ins-scripts` 到真机、再回到本地浏览器验证的完整链路与坑。

## 链路

```text
src/instagram
  → build.mjs
  → GraceChat-Earn-Android/haven/src/main/assets/scripts/instagram/<group>/
  → JsScriptManager.ASSET_SCRIPT_ORDER
  → ./gradlew :haven:assembleCommonDebug
  → adb install -r
  → app 原生 Debug → Test Ins Chats View（小崽 WebView）
  → WsDebugServer :8088（adb forward 到本机）
  → 本地浏览器打开帖子复查
```

## 已固化的 skill

`sitin-next/.claude/skills/ins-script-device-test/`，包含 `preflight` / `sync-android` /
`build-install` / `open-debug` / `ws-exec` / `verify-post` 六个脚本。

## 关键坑

### 1. WebView 没网 → `chrome-error://chromewebdata`

症状：脚本全部报 `Failed to read the 'sessionStorage' property from 'Window': Access is denied`，
`checkLoginStatus` 报 cookie 读取失败，`dom_snapshot.url=chrome-error://chromewebdata/`。

国内网络下 Instagram 直连不通。用本机代理 + adb reverse：

```bash
adb reverse tcp:7897 tcp:7897
adb shell settings put global http_proxy 127.0.0.1:7897
```

`adb shell curl` 不会自动读 Android 全局代理，验证时要显式 `-x http://127.0.0.1:7897`。
设置后要 force-stop app 再开，已有 WebView 不会自动切换代理。

### 2. app 显示 `Authorize / Authorization expired`，但 WebView 已登录

这是 app 的**服务端授权**状态，不是 IG WebView 的登录态。以 WS 里的 `checkLoginStatus` 为准：

```json
{"isLoggedIn":true,"userId":"43150596056","username":"shangbin2026","source":"cookie"}
```

`isLoggedIn:true` 就能测脚本，不必先点 Authorize。

### 3. WS Debug Console 的 request_id 不是客户端传的

`WsDebugServer.handleBrowserCommand` 自己生成 `dbg_<8hex>`，并忽略客户端请求里的 id。
新连接还会补发历史 `dbg-send` / `dbg-result`，按 action 名匹配会拿到旧结果。

正确做法：监听 `dbg-send`，按 `action + platform + params` 找到本次命令，取它广播的
`request_id`，再等同 id 的 `dbg-result`；历史消息用外层 `ts` 和 `CONNECTED.server_time` 过滤。

### 4. 刚打开就执行 → `Cannot read properties of undefined (reading 'execute')`

WS 端口起来不等于 WebView 脚本挂载完。要等 logcat 出现：

- `Page ready: instagram` / `onPageFinished, url: https://www.instagram.com/`
- `executeAction 已挂载`

`open-debug.mjs` 会 `logcat -c` 后等这两条再返回。

### 5. 测试视图冷启动后 ~12s 被自动关闭

`Test Ins Chats View` 是调试入口；冷启动后 PWA 的异常页检查超时会调用
`SocialProxyFloatingView.finish(platform)`，小崽 WebView 连同 WsDebugServer 一起销毁。
`open-debug` 成功后要**立刻**执行 action，不要开完视图去干别的。真实任务链路不受此影响。

### 6. `uiautomator dump` 返回 137 但文件已写入

MIUI 上常见：stdout 有 `UI hierchary dumped to: /sdcard/ui.xml`，进程随后被杀，exit code 137。
不要把非 0 当失败，直接 `cat` 文件即可。

### 7. 原生 Settings 和 PWA Settings 是两套

Me → Settings → Debug 是 PWA 的 vConsole 调试页，开不出小崽 WebView。
原生入口：主界面右侧蓝色悬浮按钮（`floating_btn`）→ 长按 Settings 标题 → Debug →
`Test Ins Chats View`。`floating_btn` 只在该 app 的 isLocalTest/debug 包可见。

### 8. 新脚本漏登记 `ASSET_SCRIPT_ORDER`

assets 里的 `.js` 要进 `JsScriptManager.ASSET_SCRIPT_ORDER` 才会按 build 顺序注入；未登记的按
字母序排到 `executeAction` 之后，action 还没挂载就被调度器初始化。`sync-android.mjs` 会按
`build.mjs` 的 GROUPS 重写并回读校验。

### 9. `checkLoginStatus` 返回 `isBanned:true` / `/accounts/suspended/`

`type:2`、`isBanned:true`、`path:/accounts/suspended/` 是 IG 账号级风控，不是脚本异常。
2026-10-27 测试账号 `shangbin2026` 在连续点赞/评论复测后出现该状态；停止执行、换测试号/申诉，
不要在同一账号上连续重试，真实链路也不应继续派任务。

### 10. `likePost` 的 `alreadyLiked` 恒为 false

API 版没有先查当前状态，`result.alreadyLiked` 永远是 false。判断是否重复点赞用
`hasLiked` + 本地页面 `likesDelta` / 已登录 profile 的 `likeState`。`commentPost` 没有去重，
重复调用会产生多条评论，测试用唯一文案。

## 2026-10-27 实测记录

- 分支 `feature/sp-ins-comment-like-api`，`commentPost` / `likePost` API/pipeline 版。
- 设备 `com.gracechat.prod` v1.48-test（commonDebug），IG `shangbin2026`。
- `likePost` 首次 882ms，`hasLiked:true`，帖子赞数 0 → 1。
- `commentPost` 首次 1149ms，`commentId=18009478151786627`；复测 1653ms，
  `commentId=18153527230534882`；公开页评论数 1 → 2，`og:description` 同步。
- 测试帖 `https://www.instagram.com/p/Degh6sdE_Sa/`；两条测试评论可用 IG 页面手动删除。
- **后续状态**：复测 `checkLoginStatus` 时 `shangbin2026` 返回 `isBanned:true` /
  `path=/accounts/suspended/`，账号被 IG 风控；下次真机测试先换测试号。

## 2026-10-27 补充：只有 sessionid 时如何登录 app 内 WebView

- 进入 app 后先拿 PWA WebView 的 devtools socket：
  `adb shell cat /proc/net/unix | grep webview_devtools_remote`；`adb forward tcp:9223 localabstract:webview_devtools_remote_<pid>`。
- 用 Playwright `connectOverCDP('http://127.0.0.1:9223')` 连上 PWA 页面，`context.addCookies` 写
  `.instagram.com` 的 `sessionid`（值保持 `%3A` 原样）和 `ds_user_id`。app 内所有 WebView 共享
  `CookieManager`，小崽 WebView 会直接复用登录态。
- 写完后用 `open-debug.mjs --ensure-proxy 7897 --no-restart` 打开 `Test Ins Chats View`，
  先跑 `checkLoginStatus` 确认 `isLoggedIn:true` / `isBanned:false`，再跑目标 action。
- **不要**为了复查再把这个 sessionid 登录到本地第二个浏览器：同一 session 在多处登录很容易触发
  IG `Confirm you're human` / CAPTCHA，之后 app 内 WebView 和 API 都会被重定向到
  `/accounts/suspended/?next=...`。关注态直接在 app 内再跑 `getProfileInfo` 读
  `friendship_status` 即可。
- 若已触发 CAPTCHA：停止自动重试，把设备交给账号本人完成验证；完成前不要继续跑写操作。
  2026-10-27 的 `followUserById` 真机验证就因此留下了 `@instagram` 的关注未取关。
- 实测结果（账号 `xuzaixing2026` / uid `15059533160`）：
  `getProfileInfo` source=`template`；`followUserById` 首次 `followed:true, alreadyFollowing:false,
  userId:25025320`，10.1s；二次 `alreadyFollowing:true`；复查 `friendship_status.following:true`。
