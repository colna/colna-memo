---
title: Snapchat Web 注入脚本（app-ins-scripts）踩坑与约定
date: 2026-07-21
tags: [troubleshooting, snapchat, social-proxy, sitin-next, automation]
---

# Snapchat Web 注入脚本（app-ins-scripts）

## 真源在哪（先搞清这个，别改错仓库）

```
sitin-next/packages/app-ins-scripts/
  src/snapchat/{actions,methods}/*.js   ← 真源，改这里
  scripts/build.mjs                     ← esbuild 按 group 合并
  dist/snapchat/automation.js           ← 产物，不入 git
  dom/snapchat/*.html                   ← 真实 DOM 快照（重要资产，见下）
```

⚠️ `social-proxy-scripts-container-app` 仓库里的 `scripts/snapchat/automation.js` **只是拷进去的产物**，改它没用。判断依据：bundle 顶部有 `// src/snapchat/actions/utils.js` 之类的路径注释，而该仓库 `package.json` 没有任何生成 `scripts/` 的构建步骤。

> **找落点的第一个动作是全仓 grep 能力名 / 近义词，不是先找项目目录。** 我曾在容器 App 里从零写了一个 `setChatNotification`，而正确答案是 IG 侧早有同名 `clickMute`、方案文档里连参数都定好了——整份白写。

## 既有约定（照抄，不自创）

- `SP.execute(action, params, requestId)` 直接查 `SP[action]` → **挂到 `window.SocialProxy.<name>` 即可，无需注册表**
- 多步流程用 `SP.pipeline({actionName, stateKey, ttl, ctx, validateCtx, steps, result})`，**自带 sessionStorage 断点恢复**（页面跳转后 recovery 接着跑）
- 返回值：`utils.buildResult(ACTION, data)` / `utils.buildError(ACTION, code, msg, {selector})`，都是 **JSON 字符串**
- 日志埋点：`bridge.log(level, msg)` / `bridge.eventTrack(name, params)`；bridge 依赖 `window.AndroidBridge`，不在时只 warn 不崩
- 多语言文案：`utils.LABELS` + `utils.matchLabel`
- 导航步骤模式：`navigateToChat` 返回 `data.pending` 时 **return 该结果暂停 pipeline**，等 recovery
- 重入守卫：`_running` + `try { return await SP.pipeline(...) } finally { _running = false }`

## ⭐ `dom/snapchat/*.html` 是这个包最被低估的资产

14 份真实 DOM 快照。从中可直接确证（不用猜）：

| 结构 | 用途 |
|---|---|
| `button[aria-label="<用户名>"]` | 用户/头像按钮 |
| `button[aria-haspopup="listbox"]` + 内含 `<img>` | 顶部账号菜单入口 |
| `div[role="listitem"]` | 会话列表项 |
| `div[role="gridcell"]` | 搜索结果卡片（内含 displayName / username 两个 span） |
| `div[role="textbox"][contenteditable]` / `placeholder="Send a chat"` | 聊天输入框 |
| `div[aria-label="媒体内容"]` + `div[role="slider"]` | story 播放器与页数 |

**快照缺哪些**（这几处的选择器目前只能靠文案，是最脆的部分）：

- [ ] 会话设置面板（Message Notifications / Silent）
- [ ] **自己的 story 播放器** —— `story.html` 是**好友** story 的（"查看者/浏览量/删除/保存" 关键词均 0 次）
- [ ] 中文界面的 Add Friends 弹窗（搜索框 placeholder 的译文）

> 拿快照的方法：目标 UI 打开时抓 `document.body.innerHTML` 存进 `dom/snapchat/`。

## 已知缺陷与陷阱

### 1. ⭐ `utils.simulateClick` 一次调用触发 2~3 遍

它 dispatch click 之后还调 `el.click()`，再直接调 React `props.onClick`。生产日志实证：**一次 Add 点击产生两条 `permissions.query('camera')`**。

- 已在 `followUserById` / `clickMute` / `getStory` 内部改用单次 dispatch 绕开
- **`utils.simulateClick` 本身未修** —— 其它 IG / SC 脚本仍在双触发，改它影响两个平台，待单独立项
- 危害与原理详见 [[ui-automation-selectors]]

### 2. `utils.captureDomSnapshot` 的 `querySelector` 会炸

它对传入的 `selector` 直接 `document.querySelector(selector)`。而调用方常传**给人看的定位描述**（`"text:Silent"`、`"span:contains(...)"`）——这些不是合法 CSS，抛 `SyntaxError`。

因为它跑在 `buildError` 内部，**整条失败路径的真实错误码会被替换成 `EXCEPTION`**，DOM 快照也一并丢失。

- 已加 try/catch 并置 `selector_invalid` 标记
- 调用方一律传合法 CSS，"找的是哪句文案"写进 message

> **这类 bug 只在失败路径触发，正常流程永远碰不到** —— 所以能一路合并上线。**只测 happy path 等于没测错误处理。**

### 3. ⭐ `class="nonIntl"` 的文案**照样会翻译**

`LABELS` 头注释原先写着「nonIntl 固定英文（如 "Add Friends" 标题、`placeholder="Search..."`、"My AI"）不在此列，直接写死即可」——**这条规则是错的**。中文界面实测失败提示是「出错了。」。

- 已连根修正，并把它点名的三个写死项标为待核实
- **仍写死英文的**：`placeholder="Search..."`（followUserById Step 1，非英文界面下一直落空，靠第二个选择器兜底）、"Add Friends" 标题
- 没有对应语言的 DOM 就**不要瞎猜译文** —— 猜错等于加一条静默失效的选择器

### 4. story 播放器的四个微妙点

写在 `getStory.waitForViewer` / `collectPages` 里，**两个 story 方法共用**：

- 预加载期页面上会同时存在**多个** `aria-label="媒体内容"` 容器（含空壳与上一条 story 残留）→ 优先取真正含 slider 或媒体的那个
- slider 与 `<img>/<video>` **晚于**媒体区渲染 → 只读一次极易拿到 0 页而误报
- **单页 story 根本没有 slider** → 无 slider 时按有无媒体算 1 页
- 翻页必须**严格前进一页** → 不能用 `simulateClick`

另外「播放器是否已打开」的判据要用 `getTotalPages() > 0`，**不能只看容器存在** —— 容器可能是空壳或上一条的残留。

### 5. 方法命名与加载顺序

- `getStory` = 拉**自己**的 story；`getFriendStory` = 拉**好友**的（2026-07-21 改名，管道侧 `FETCH_STORY` 对应后者）
- 播放器读取函数挂在 `getStory.waitForViewer` / `getStory.collectPages` 上，`getFriendStory` 从 `SP.getStory` 取 → **`m:getStory` 必须在 `m:getFriendStory` 之前**，顺序错了引用是 undefined
- ⭐ 验证跨文件引用的省事办法：调用它并断言**返回的不是 `EXCEPTION`** —— 引用断了必然 EXCEPTION，正常流程会走到真实业务错误码。不用去窥探闭包内部。

### 6. 参数口径：文档与实现不一致（未决）

| 方法 | 文档 | 实现 |
|---|---|---|
| `clickMute` | `{snapId}` | 沿用 `chatId ?? snapId` 兜底 |
| `getFriendStory` | `{snapId?}`，「无 snapId 拉全量」 | **必传**，否则 `INVALID_PARAMS` |

`navigateToChat` / `sendMessage` / `getChatMessages` 实际都吃 **chatId（聊天 UUID）**，而文档 §7.2 多处写 `snapId`。**服务端到底下发哪个待定**。

## 工程环境

- **该包不参与 lint**：`package.json` 无 lint 脚本，`turbo lint` 直接跳过。既有文件普遍报 `no-undef` / `max-lines-per-function`
- **仓库 CLAUDE.md 要求 English only**（代码/注释/UI 文本），但既有 SC 脚本注释与日志大量是中文 —— 新代码写英文会导致同文件中英混排，是否整体英文化未决
- **pre-commit** = `pnpm lint` + `pnpm circular`；**commit-msg** = commitlint，**header ≤ 100 字符**（超了只报 `commit-msg script failed (code 1)`，不说哪条规则）
- **pre-push** = 全量 `pnpm test`，会被 `business-minerva-upgrade` 挡住（`@presence-io/datatester` 所在私有 registry `nexus.sitinai.com` 需内网）。**turbo 会把被中断的其它任务一并标记 ELIFECYCLE** —— 逐包 `--filter` 单跑才能分清「真失败」与「被连坐」
- `git cherry-pick` **不触发 pre-commit hook**，lint / circular / build 要手动补跑

## notifyPageAbnormal 上报走 `window.JSBridge`,不是 `AndroidBridge`(2026-08-03 核对端源)

登录态异常上报(`checkLoginStatusIIFE.js` 的 `notifyAPK`)**必须走 `window.JSBridge.notifyPageAbnormal`**,不能用 `window.AndroidBridge.notifyPageAbnormal`。核对 `GraceChat-Earn-Android` 端源结论:

- `notifyPageAbnormal` 原生实现 + 注入在 **`PWAWebViewFragment.kt`**:端为「social-proxy 异常检测 WebView」`addJavascriptInterface(object{@JavascriptInterface notifyPageAbnormal(payload)}, "JSBridge")`。**checkLoginStatusIIFE.js 就跑在这个检测 WebView**,所以 `window.JSBridge.notifyPageAbnormal` 是真·原生能力(端收到后据 `type≠0` 定向关平台会话)。
- 抓取用的 SocialProxy WebView(`WebViewManager.kt`)**只注入 `"AndroidBridge"`**;`AndroidBridge.kt` 的 @JavascriptInterface **没有 notifyPageAbnormal**(只有 onResult/scriptEvent/log/request/event/finish/changeStepText/requestHttp/getLastMessageId/uploadMessages/isFullSyncDone/setFullSyncDone)。→ 用 AndroidBridge 上报永远走 `else` 打 warn、异常态漏报。
- **配套铁律**:`bridge.js` 结尾**不要**再写 `window.JSBridge = window.SocialProxyBridge`(+ `.response = _onResponse`)。那句会把端的原生 JSBridge 覆盖成 JS 包装层 SocialProxyBridge(无 notifyPageAbnormal)。GraceChat ca9fe3f9 已把它注释掉,sitin-next 同步注释。
- ⚠️ GraceChat/旧版 `checkLoginStatusIIFE.js` 头注释「notifyPageAbnormal 只挂在 AndroidBridge 上」是**过时残留**,与其自身代码矛盾,别照信。

## 首次登录态检测必失败、再查才好(2026-08-03)

**现象**:GraceChat 首次检查 SC 登录态一定 `type:1`(上报 notifyPageAbnormal 关会话),再查一次就正常。

**根因**(`checkLoginStatusIIFE.js` + `checkLoginStatus.js`):
1. 检测窗口太短:IIFE 原本 `INITIAL_DELAY=1000 / MAX_TRIES=3 / POLL_INTERVAL=1000`,总窗口 ~3s。`/v2/welcome` 是 Snapchat Next.js SPA,登录卡片渲染晚于 3s(日志会有 `Abort fetching component for route "/v2/welcome"` = 页面还在重载路由),3 次轮询全落空。
2. 判定缺陷:`checkLoginStatus` 只有登录态正向信号,找不到就一律 `type=1`,**「页面没渲染好」= 「未登录」不可区分**。

**端超时(重要数字)**:`PWAWebViewFragment.doCheckSocialProxyAbnormalWebview` 的 `SocialProxyPageCheckRequest.DEFAULT_TIMEOUT_MS = 30_000L`。检测 WebView 存活 30s;JS 若不在 30s 内上报,端 postDelayed 兜底以 `type:-1 error:timeout` **照样 finish 关会话**。→「不上报」不安全,但有 ~27s 空间可用。

**修法**:窗口拉到 25s(< 30s)、间隔 500ms、`type=0` 命中即返回;新增正向登出信号 `isLoggedOut`(密码框/用户名输入/登录表单),且需**连续稳定 3s** 才判 `type=1`——规避已登录用户 `/v2/login→/v2/welcome` 跳转途中短暂闪现登录表单被误判。窗口耗尽才报 1。

> 检测 WebView **只注入 JSBridge、无 AndroidBridge**,故 `bridge.eventTrack/log` 无效(`AndroidBridge.event 未注入`),`sp_check_login_status` 埋点在该 WebView 丢失。

相关：[[ui-automation-selectors]] · [[social-proxy-scripts-container-app]] · [[sitin4-endchat-backend-gap]] · [[android-webview-multi-social-memory]] · [[pwa-video-call-native-bridge]]

## SC 页面 console 被中和 → 用隐藏 iframe 的干净 console(2026-08-12)

Snapchat 页面改写/中和了 `console`,直接 `console.log` 不显示。脚本 `bridge.log`(`snapchat/actions/bridge.js`)的解法:建一个隐藏 iframe,取 `iframe.contentWindow.console`(干净原生 console),缓存为 `window.__spConsole`,输出带 `%c[SC-Scraper]` 橙色前缀,同时 `AndroidBridge.log` 发原生。
- 手动调试:脚本加载后直接 `window.__spConsole.log(...)`;或自己建 iframe 取 console。
- 诊断脚本统一 `var C = window.__spConsole || console;`。

## getChatMessages 提取 0 条:消息 li 内的 `<time>` 被误判成日期分隔符(2026-08-12)

`getChatMessages.js` 提取循环用 `li.querySelector("time")`(后代搜索)判断日期分隔符。Snapchat 每条消息 li 内部带消息时间戳 `<time>`(如 16:08),后代搜索命中 → 消息 li 全被当日期分隔跳过 → 提取 0 条(但「气泡总数」正常,因为那是后代 `div[style*=border-color]` 计数)。
- **判据**:日期分隔 li 的 `<time>` 是**直接子**(`:scope > time` 命中);消息 li 的 `<time>` 是**深层**。
- **修法**:`li.querySelector(":scope > time")` 只认直接子。commit 71a11443e。
- 通用教训:SC DOM 抽取区分「结构节点 vs 内容节点」优先用 `:scope >` 直接子,避免后代搜索串味。

## SC 消息抽取 / 增量 / DM 上报 一批坑(2026-08-12)

**1. 提取 0 条:消息 li 内的 `<time>` 被当日期分隔符**
`getChatMessages` 用 `li.querySelector("time")`(后代)判日期分隔符。SC 每条消息 li 内带时间戳 `<time>`(16:08)→ 消息 li 全被当分隔符跳过 → 0 条(但「气泡总数」正常,那是后代 `div[style*=border-color]` 计数)。**修:`:scope > time` 只认直接子。** 日期分隔 li 的 time 是直接子,消息的 time 是深层。

**2. senderName = 时间**
`parseSenderFromHeader` 倒序取最后一个叶子 span,现在 header 末尾是 `<time>` → senderName 变"16:08"。**修:跳过 `<time>` 内 span + 纯 `HH:MM` 文本**,名字在时间之前。

**3. dateLabel 空(端字段名不一致)**
脚本消息发 `date`,端 `SnapMessageDto` 读 `dateLabel` → Gson 对不上,端落库/上报 CHAT_HISTORY 日期空。**修:脚本补 `dateLabel=date`(保留 date)。** 教训:改字段前核对端 DTO 字段名。

**4. 增量:端已备 orderKey 游标,脚本要主动去拿**
端 `getLastMessageId`:IG 返回最新消息 id,SC 返回 `getLastOrderKey`(最新 orderKey 数值序,空 orderKey 不参与)。脚本 `filterByCursor` 用 `BigInt(orderKey) > 游标`。**IG 脚本调 getLastMessageId,SC 之前没调** → 后端不传游标就每次全量。修:SC 也 `lastOrderKey = params.lastOrderKey || bridgeGetLastMessageId(chatId)`。三层去重:脚本 orderKey 游标 → 端 Room 按 id upsert → 后端按 id。

**5. NEW_MESSAGE 双报:typing 被当未读**
DM listener `diffAndReport` 逻辑:未读下 status/timestamp 变化就重报。打字指示器"输入中…"被 `isItemUnread`(嵌套三层启发式)判未读 → 先报一次;真消息 status 变再报一次。**修:`utils.notUnreadStatus` 加 typing 变体("输入中…/正在输入…/Typing…/Escribiendo…",省略号 `…` 和三点 `...` 都列,matchLabel 是精确比对)。**

**6. 端 uploadMessages → CHAT_HISTORY 链路(端不是转发脚本 result)**
`AndroidBridge.uploadMessages` → 落 Room(id 去重 / snap_chat_state 缓存双方身份 / 媒体落盘)→ `ActionDispatcher.startUploadSnap` 组批(200/批,媒体传 OSS)→ 组 CHAT_HISTORY(顶层身份取自 snap_chat_state)→ SCRIPT_EVENT 发后端 → ACK 后 markUploaded。script getChatMessages 的 onResult result 缺 `interaction:true` 会被后端 shouldRoute 挡,但消息走这条独立通道不丢。

**7. 目标会话解析(getChatMessages / sendMessage 通用)**
优先级 chatId(threadId)> manSocialNickname[> 第一个未读]。先判当前窗口是否已在目标(chatId 命中 URL `getChatIdFromUrl()` / nickname 命中顶栏 `[aria-haspopup="listbox"]`),在则跳过,否则 chatId→navigateToChat(整页刷新,pipeline finally 清 state 后从 step0 幂等重跑)/ nickname→搜索框输入+点卡片(SPA 不刷新)。忽略名单用 `SP.isIgnoredContact`(startDMListener 暴露,共享一份)。
