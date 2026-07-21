---
title: Chrome 扩展的自动化测试（--load-extension 已被禁用后怎么办）
date: 2026-07-21
tags: [troubleshooting, chrome, extension, e2e, cdp, mv3]
---

# Chrome 扩展的自动化测试

## 1. ⭐ Chrome 136+ 禁用了命令行 `--load-extension`，且**静默失败**

实测 Chrome 150（2026-07）：`--load-extension` + `--disable-extensions-except` 出现在 `chrome://version` 的命令行里（说明 flag 被接受），但 `chrome://extensions` **列表为空**，没有任何报错。

试过且**无效**的绕法：

- `--disable-features=DisableLoadExtensionCommandLineSwitch`
- 改用 `--remote-debugging-pipe` 代替 `--remote-debugging-port`（这是 Puppeteer 的官方绕法，对本版本无效）
- 预置 `<user-data-dir>/Default/Preferences` 打开 `extensions.ui.developer_mode`
- 关掉 `--headless=new` 用 headful

**根因排查手法（值得复用）**：写一个 3 行的最小 MV3 扩展（`manifest.json` + 一个空 `bg.js`）用同样命令加载。最小扩展也加载不了 → 是浏览器限制；能加载 → 是自己 manifest 的问题。这一步把「我写错了」和「它不让我干」分开，比盯着自己的 manifest 猜快得多。

## 2. 绕法：chrome API 桩 + 真实 CDP 输入事件

既然装不进扩展，就把**未经改动的** `content.js` 用 `<script src>` 直接加载进普通页面，页面里先铺一个 `window.chrome` 桩：

```js
window.chrome = {
  runtime: {
    sendMessage: (msg) => { collected.push(msg); return Promise.resolve({ ok: true }); },
    onMessage: { addListener: (fn) => listeners.push(fn) }
  }
};
```

再手工把 `{type:'BR_START_RECORD'}` 之类的消息 dispatch 给收集到的 listener。

**关键是不要为测试改动源码** —— 桩的是环境（`chrome.*`），跑的是真源码。一旦为了可测性改 `content.js`，测的就不是线上那份了。

配合 CDP `Input.dispatchMouseEvent` / `Input.insertText` 发**真实 `isTrusted` 事件**，验证的才是「真实用户操作能否被录到」，而不是「我们自己派发的合成事件能否被自己收到」——后者是自证循环。

覆盖不到的只剩 `background.js` 的存储与跨页调度，如实写进 README，别假装测了。

## 3. CDP over pipe 的实现要点

`--remote-debugging-pipe` 时协议走 fd 3/4，不是 WebSocket：

```js
spawn(CHROME, args, { stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] })
// stdio[3] = 我们写给 Chrome，stdio[4] = Chrome 写给我们
// 报文之间以 \0 分隔，不是换行
```

没有 `/json/list` HTTP 端点，改用 `Target.setDiscoverTargets` + `Target.getTargets`。

**MV3 service worker 是懒启动的**：没有事件唤醒时它压根不在 target 列表里。先打开一个会注入 content script 的页面把它唤醒，再去找 —— 否则会把「SW 在睡觉」误判成「扩展没加载」。

## 4. 零依赖的 DOM 单测：`chrome --dump-dom`

不想装 jsdom / playwright 时，把断言写进一个 HTML 页面、结果渲染到 `<pre id="result">`，然后：

```bash
chrome --headless=new --allow-file-access-from-files --virtual-time-budget=3000 \
       --dump-dom "file:///path/test.html" | 抓 <pre id="result">
```

跑的是真 Chrome 的真 DOM（`CSS.escape`、`getRootNode`、shadow DOM 全都是真的），比 jsdom 更可信，且零依赖。

> 本机踩到的环境坑：`npm i -D jsdom` 反复输出 `up to date`，但 `node_modules` 根本不存在（无报错）。删 lock 重装、关沙箱都无效。别在这上面耗时间，直接换零依赖方案。

## 5. 断言要写在「结果」上，不要写在「过程」上

这次真正抓到 bug 的断言只有一条：**回放后表单的 summary 字符串 === 录制时的 summary 字符串**。

「录到了 N 步」「有 click 类型」这类过程断言全绿，但其中混着一步错误的 `input value="yes"`（点 checkbox 也会派发 `input` 事件，`el.value` 是 value 属性不是用户输入）。是「连续输入应合并成 2 步、实际 3 步」这条**具体到值**的断言把它逼出来的。

**过程断言只能证明「跑过了」，结果断言才能证明「跑对了」。**

相关：[[headless-screenshot-spa-cdp]]
