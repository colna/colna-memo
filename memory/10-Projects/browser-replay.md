---
title: browser-replay
date: 2026-07-21
tags: [project, chrome-extension, mv3, automation, colna]
---

# browser-replay

浏览器操作录制 / 回放 Chrome 扩展（MV3）。录制点击 / 输入 / 聚焦 / 选择 / 勾选 / 按键 / 滚动 / 导航，回放时自动重做，可导出 JSON。

- 仓库：`git@github-colna:colna/browser-replay.git`（**个人仓库**，非 presence-io）
- 本地：`/Users/max/Dev2/zhangzheng/browser-replay`
- 起点：2026-07-21 从空仓库建起，当天推了 3 个 commit

## 硬约束：选择器不含 class

这是提出需求时就定死的。定位优先级与完整方法论见 [[../30-Resources/troubleshooting/ui-automation-selectors]]。

## 结构

```
manifest.json          MV3
src/lib/selector.js    选择器生成/解析（最核心）
src/lib/waiter.js      等待策略
src/lib/executor.js    动作执行
src/lib/export.js      导出格式（抽出来才能在 node 里断言）
src/content.js         录制器 + 回放执行器，不持有任何进度状态
src/background.js      SW：录制会话 + 回放游标的唯一权威，状态全落 storage.session
src/popup/             列表 / 步骤查看 / 导出 / 导入
test/                  selector 单测 + 端到端
```

**架构要点**：进度只属于 background。页面一跳转 content script 就被销毁重建，谁把游标存在页面侧，谁就会在跳转后错乱。

## 测试

`npm test` —— 需要本机 Chrome，**零 npm 依赖**。

- `test/selector-test.html`（11 项）：真实 DOM 上验证无 class、动态 id 不采用、结构路径从地标起算、候选唯一命中、shadow DOM 穿透、结构失效后靠文本兜底
- `test/e2e.mjs`（24 项）：headless Chrome + CDP `Input` 发**真实 isTrusted** 鼠标键盘事件录制 → 回放 → 断言表单数据完全一致

> E2E **没有加载扩展本体**：Chrome 136+ 已禁用命令行 `--load-extension`。改用 chrome API 桩把未经改动的 `content.js` 装进普通页面跑。详见 [[../30-Resources/troubleshooting/chrome-extension-e2e-automation]]。

## 已知限制

- 事件是 DOM 合成的（`isTrusted=false`）：触发下载 / `window.open` / 原生文件选择会被浏览器拒绝。需要时把 `executor.js` 换成 `chrome.debugger` 的 CDP 后端（代价：顶部常驻调试横幅）
- 拖拽、原生 `<select>` 下拉层内的点击未覆盖
- 回放只在单个标签页内进行，不跟随新开的标签页

## 待办

- [ ] `background.js` 的存储与跨页面调度**未被自动化覆盖**（同上，扩展装不进测试环境）。需手动装载扩展验一次：录一段**带页面跳转**的流程 → 回放看游标能否续跑
