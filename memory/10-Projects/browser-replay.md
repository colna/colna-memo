---
title: browser-replay
date: 2026-07-21
tags: [project, chrome-extension, mv3, automation, colna]
---

# browser-replay

浏览器操作录制 / 回放 Chrome 扩展（MV3）。录制点击 / 输入 / 聚焦 / 选择 / 勾选 / 按键 / 滚动 / 导航，回放时自动重做，可导出 JSON。

- 仓库：`git@github-colna:colna/browser-replay.git`（**个人仓库**，非 presence-io）
- 本地：`/Users/max/Dev2/zhangzheng/browser-replay`
- 起点：2026-07-21 从空仓库建起，当天推了 3 个 commit；7-22 又推 3 个（祖先快照、自绘编辑器修复、祖先属性全收）

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

`npm test` —— 需要本机 Chrome，**零 npm 依赖**。当前 **46 项**。

- `test/selector-test.html`（15 项）：真实 DOM 上验证无 class、动态 id 不采用、结构路径从地标起算、候选唯一命中、shadow DOM 穿透、结构失效后靠文本兜底、祖先链快照与属性
- `test/e2e.mjs`（31 项）：headless Chrome + CDP `Input` 发**真实 isTrusted** 鼠标键盘事件录制 → 回放 → 断言表单数据完全一致；另有一组自绘编辑器场景（与 Snapchat 同构的 fixture），**断言落在业务结果**（消息是否真的发出）而非步骤执行状态

> E2E **没有加载扩展本体**：Chrome 136+ 已禁用命令行 `--load-extension`。改用 chrome API 桩把未经改动的 `content.js` 装进普通页面跑。详见 [[../30-Resources/troubleshooting/chrome-extension-e2e-automation]]。

## 已知限制

- 事件是 DOM 合成的（`isTrusted=false`）：触发下载 / `window.open` / 原生文件选择会被浏览器拒绝。需要时把 `executor.js` 换成 `chrome.debugger` 的 CDP 后端（代价：顶部常驻调试横幅）
- 拖拽、原生 `<select>` 下拉层内的点击未覆盖
- 回放只在单个标签页内进行，不跟随新开的标签页

## 导出的 JSON

每步除选择器外还带**往外最多 10 层的祖先结构快照**（`depth` 从 1 起算 = 直接父）。记的是每层祖先「自己」是什么（tag / nth-of-type / childCount / 直接文本 / 属性），**不是 outerHTML** —— 第 10 层的 outerHTML 往往就是大半个页面，没法读也没法 diff。

属性**除 class / style 外全收**（含 `aria-expanded`、`aria-selected`、`tabindex`、站点自定义 `data-*`）：白名单永远追不上各家站点，而这些状态位正是判断「当前是展开还是选中」的唯一依据。id 仍按稳定性判定后才记。

## 待办

- [ ] `background.js` 的存储与跨页面调度**未被自动化覆盖**（同上，扩展装不进测试环境）。需手动装载扩展验一次：录一段**带页面跳转**的流程 → 回放看游标能否续跑
- [ ] **祖先快照占导出体积的大头**（7-22 实测一份 15 步的 SC 录制里占 58%，11 条祖先链去重后只剩 4 条）。改成全量取属性后还会涨，未实测。可做的：按步去重 / 引用化
- [ ] 真实站点上点按钮内的 `svg` 时**没上溯到 button**（停在 11 层结构路径）。`describeClickTarget` 的判据是 `altScore > directScore`，两者同为 structural=40 时不换。**fixture 里没能复现**，差异未查清，故未动逻辑
- [ ] 录制会把原地不动的 `scroll`（`0,0`）也记进去，纯噪声

---

## 2026-07-24 进展:逐键录制 + Instagram DM 实战调优

一天 6 个 commit,版本 **0.1.0 → 0.2.0**。前半天做逐键功能,后半天全部花在「让一份 Instagram DM 的真实录制能跑通」上 —— 这个过程本身比功能更有价值,踩到的坑已抽到 [[../30-Resources/troubleshooting/ui-automation-selectors]] 第四、五节。

### 功能

- `6790be3` **逐键录制 / 回放**。新增 `src/lib/keyboard.js`(纯文本编辑模型 `applyKeystroke`,录制端影子缓冲与回放端共用)。只对「选区安全」的文本框(`input[text/search/tel/url]`、`textarea`)逐键;contenteditable / number / email / 敏感字段一律走值快照。回放用合成事件 + 原生 setter 手动插值(**未接 chrome.debugger**,仍 `isTrusted=false`)。IME 走 `compositionend` 整段插入。

### 定位质量(用 IG 真实录制量化)

改前:9 个非导航步骤只有 **1 步**是强候选,8 步靠 12 层裸 div 吊着。改后 **7 步**是语义候选。

- `5d4a714` 白名单加 `aria-placeholder` / `contenteditable`;新增 `identityDataAttrs()` 收站点自有 `data-*`(排除状态位与 hash 值);`role` 只参与两两组合;**`selfAnchor` 判据改为「值稳定 + 文档内唯一」**,于是 `[data-pagelet="..."]` 能当结构路径起点(这条收益最大)
- `c98e7bd` 结构路径不唯一时退到逐层 `:nth-child`(封顶 12→30);零候选时无条件保留一条
- `5ce300d` 弱候选命中后**必须**过「像不像」校验(tag / aria-label / placeholder),不像就报错停下

### 关键实测教训

1. **「找不到」≠「找错了」**。IG 输入框为空时发送按钮原地变麦克风,结构路径按位置命中它 → 回放变成「开始录音」。我一度给校验加了「都不像就退回第一个匹配」的后路,是错的。
2. **排查「改了没生效」前先证明代码版本**。`content.js` 的注入哨兵是布尔量,扩展重载后已打开的页面里新代码**一行都不跑就 return** —— 为此白排查两轮,还据错误现象下过一个错误结论。`c30f45d` 改成版本号哨兵 + `ensureInjected` 比对版本 + 导出加 `contentVersion`。
3. **要分清「没观测到」和「观测到但被判定为无需记录」**。`7d508db`:IG 恢复草稿,内容在聚焦前就在框里,被 `onFocusIn` 当成基线 → 失焦时判定「值没动过」静默跳过。修法:contenteditable 的聚焦基线固定为空串。

### 未完成:IG 端到端仍未验证通过

用户最后发来的录制 id 仍是 `br_mryl8xj1_76rxar`(与上一份同一次录制、**无 `contentVersion` 字段**),即产出它的 content script 早于 `c30f45d`。**「输入能否被录到」这个问题在真实 IG 上尚未得到一次有效验证**。用户已叫停,后续继续时:

- 先看导出 JSON 的 `contentVersion` 是否为当前版本,不是就不用往下分析
- 是当前版本仍无 `input` 步骤,则前述所有假设都被证伪,需要在页面里直接取证(在 IG 页面手动挂 window 捕获监听,看 focusin/keydown/beforeinput/input 各自是否到达)

## 待办(2026-07-24 追加)

- [ ] **IG 实战验证未完成**(见上)
- [ ] 「点 svg 没上溯到 button」这条老待办**在真实 IG 上复现了**:返回按钮录成 `[href="/colna_zheng/"] > … > span > svg`(structural 40),而 depth 3 的祖先正是 `div[role="button"]`。`describeTarget` 判据 `altScore > directScore` 在两者同为 40 时不换 —— 现在有真实样本了,可以照它搭 fixture
- [ ] 未接 `chrome.debugger` 可信按键:受用户手势保护的行为(下载 / `window.open`)逐键仍触发不了
- [ ] IME 只覆盖 `compositionend` 的提交文本,组字中途的候选切换不还原
