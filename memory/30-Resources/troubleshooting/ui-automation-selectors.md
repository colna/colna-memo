---
title: UI 自动化的元素定位与结果验证
date: 2026-07-21
tags: [troubleshooting, automation, dom, selector, snapchat, browser-replay]
---

# UI 自动化的元素定位与结果验证

来自同一天两个项目的共同经验：[[browser-replay]]（浏览器录制/回放插件）与 Snapchat 注入脚本（见 [[snapchat-web-scripts]]）。两者都在解决同一个问题——**在一个你不控制的页面上，稳定地找到元素、点对它、并确认真的生效了**。

---

## 一、定位：什么能用，什么不能用

### 1. class 一律不可用

CSS-in-JS / 原子化 CSS / 构建 hash 之下，class 几乎必然漂移。Snapchat 的真实 DOM 里全是 `XlW_1` / `AbUJt` / `Qz2mt` 这种。录制当时唯一的 `.css-1x2y3z`，下次构建就是另一个名字。

可用的定位优先级（每条生成时**当场验证唯一性**，不唯一直接丢弃）：

| 分值 | 方式 |
|---|---|
| 100 | `data-testid` / `data-cy` 等测试属性 |
| 92 | `#id` —— 但要**排除动态 id** |
| 88 | `[name]`（表单元素的天然主键） |
| 66–80 | `aria-label` / `placeholder` / `role` 组合 |
| 55 | **元素文本 / label**（非 CSS，遍历匹配） |
| 40 | `tag:nth-of-type(n)` 结构路径 |

### 2. 动态 id 要主动排除

这些 id 看着能用，实际每次渲染都变：

```
^:r[0-9a-z]+:$        React useId
[0-9a-f]{8,}          构建 hash
\d{4,}                自增序号
^(ember|radix-|mui-|headlessui-|downshift-)   组件库前缀
```

Snapchat 的头像按钮 id 是 `downshift-0-toggle-button`——**多渲染一个下拉，序号就变**。改用同元素上的 `aria-haspopup="listbox"` 才稳。

### 3. ⭐ 文本要排在结构路径**之前**

一个 6 层的 `nav > div > div > div` 只要中间插进一个包装层就全废；而按钮上的文字改版后通常还在。

踩过的坑：原来的 resolve 顺序是「先试完所有 CSS 候选、再用文本兜底」——等于让**最脆的**结构路径优先于**最稳的**文本。**兜底顺序要按可靠性排，不能按生成顺序排。**

### 4. 结构路径从地标起算，别从 body 铺

`html > body > div > div > nav > …` 砍成 `nav > …`。把 `nav` / `main` / `header` / `footer` / `form` / `table` 这类地标标签（页面里通常唯一）当路径起点。**路径每短一层，扛住改版的概率就高一截。**

### 5. ⭐ 事件的 `target` ≠ 用户的意图对象

点一个按钮，事件 target 往往是按钮里的 `<span><span>文字</span></span>`。这些包裹层没有任何标识，只能退到十来层结构路径；而外面那个按钮多半带着 `aria-label` 或 `data-testid`。

实测：`nav > div:nth-of-type(2) > … > span:nth-of-type(1) > span > span`（11 层）→ 上溯后变成 `button[aria-label="收藏此项"]`。

**规则是择优上溯，不是无脑上溯**：只在目标自身评分低（只能退到结构路径）时才向上找最近的**可交互元素**（button / a[href] / role=button / 表单元素），且新目标必须确实更好认才换。

只上溯到可交互元素、不上溯到布局容器——**回放点击取的是元素中心点，容器太大时中心可能落到别的子元素上**。

### 6. ⭐ 类名 / 字段名不是证据

Snapchat 的失败提示渲染在 `<span class="nonIntl">` 里。我据此断言「nonIntl = 不国际化 = 固定英文，写死即可」——**中文界面实测是「出错了。」**。

`nonIntl`、`isTemp`、`readonly` 这类命名只说明**当初的意图**，不说明**当下的行为**。拿它当依据下断言，和读了函数名就断言实现没有区别。

> 这条假设当时还写在公共字典的头注释里，并传染到了三处代码。**改错误结论时要 grep 它的所有副本**，只改自己写的那份等于没改。

### 7. 「精准」和「稳定」是两个问题

一个 11 层路径可以做到 100% 唯一命中（精准），同时 100% 活不过下次改版（不稳定）。用户问「这种查找方式精准吗」时，通常真正想知道的是后者——**分开回答**。

### 8. 脆弱性要在 UI 上可见

选择器分值藏在 JSON 里等于没有。标在步骤列表上（`< 60` 标「⚠ 易失效」），用户才知道该给哪几个元素加 `data-testid`——**那才是治本的动作**。

---

## 二、点击：一次就是一次

### ⭐ `dispatchEvent(new MouseEvent('click'))` 之后不要再 `el.click()`

合成 click 事件本身就走 activation behavior，两者叠加 = **handler 跑两遍**。同一天在三处独立撞见：

1. 我自己写的 fallback `click()`——测试里点击链是 `A#opt-silent → A#opt-silent`
2. 既有 `utils.simulateClick`——dispatch click + `el.click()` + 直接调 React `props.onClick`，**最多三遍**
3. 既有 story 翻页代码的注释早就写着：「一次调用直接连翻 2~3 页」

**三级证据链**（值得照这个模式取证）：

| 层级 | 证据 |
|---|---|
| 代码 | 读实现看到 dispatch 之后又 `el.click()` |
| 浏览器 | fixture 里 handler 计数 = 2 |
| **生产** | **用户日志里同一条 `permissions.query('camera')` 出现两次** |

危害按操作性质分：

- 幂等操作（发消息）——无所谓
- **开关 / 单选**——设了又撤
- **菜单 / 弹窗**——打开后立刻关闭
- **提交类**——请求在途时第二次激活到达，**服务端拒绝正是典型表现**（很可能就是 "Something went wrong." 的成因）

正确写法：保留完整 pointer/mouse 序列（`pointerdown → mousedown → pointerup → mouseup → click`），**只去掉 `el.click()`**。保留序列是因为靠 pointerdown/mousedown 打开的菜单不响应孤立的 click 事件。

---

## 三、验证：点了 ≠ 成功了

### ⭐ 点击后必须有独立的成功判据

原来的代码点完 Add 只 `log` 一下按钮状态，然后无条件返回 `added: true`。而 Snapchat 有时会拒绝请求、弹 "Something went wrong."，按钮仍停在 "Add"——**失败被记成成功**，报表上完全看不出来，只有对账时才发现数量对不上。

判据设计：

| 观察到 | 结论 |
|---|---|
| 状态变为 Added/Pending | 成功 |
| 出现失败提示 | 明确失败 |
| 超时内两者都没有 | **未确认**（也报错误，不报成功） |

三个必要细节：

1. **点击前先记录已有的失败提示并排除**——否则上一次残留的提示会让这次误判
2. **每轮重新解析目标**——点击会让该行重渲染，早先捕获的节点已 detach
3. **先判失败再判成功**——失败提示是确定信号，状态没变只是尚未确定

### 「未确认」应该报错误而不是成功

上游可以重试，而重试无害（已加过的目标会返回 ALREADY_ADDED）；报成功反而把静默失败埋进日志。**让失败朝着「用户能看见」的方向倒。**

---

## 四、录制类工具的两条

### 1. 「顺带发生的事件」不等于「用户的动作」

SPA 常给布局容器挂 `tabindex`，点击时它真的拿到焦点。于是每次点击都多出一个 focus 步骤——而这类元素没有任何稳定标识，只能退到 8 层结构路径，**是整条脚本里最先失效的部分**。

只录「聚焦本身就是动作」的元素（input / textarea / select / contenteditable）。按钮的焦点由 click 自带，布局容器的焦点不影响任何状态。

### 2. 失败处理要分级

按「这步失败会不会让后面变得没意义」分：

- `focus` / `blur` / `scroll` 失败 → 跳过继续（等待上限也该更短）
- `click` / `input` / `submit` / `navigate` 失败 → 立即中断

同一个 `if (!ok) return` 对两者都不对。另外这三类可选步骤**不该要求元素「可交互」**——布局容器塌陷成 0 尺寸很常见，但 `el.focus()` 照样有效。

### 3. 超时报错必须带「我试了什么、各匹配到几个」

「等待超时」四个字让人完全无从下手。

```
页面上找不到该元素。已尝试：#submit → 0 个匹配；nav > div > div → 0 个匹配
```

并且要区分「页面上没有这个元素」和「找到了但一直不可交互」——两者的后续动作完全不同。

---

## 五、录制脚本怎么用

**录制是「意图的证据」，不是「实现的蓝图」。**

它能可靠告诉你的：用户点了什么**文案**、按什么**顺序**、页面 **URL** 怎么变。
它给出的 DOM 路径只是那一刻的快照——照搬进自动化脚本，等于把最脆的部分固化下来。

实践中一份 13 步的录制，9 步是 score 40 的十来层 `nth-of-type`。正确做法是**从录制里提取语义、用语义重写**，并按证据分级在注释里标明：

```
已确证（DOM 快照）：…
已确证（录制）：文案与交互顺序
⚠️ 未确证（按语义推断，需真机校准）：层级、非英文文案、选中态属性
```

没有真实环境可验证时，**交付「可自检的东西」而不是「声称能用的东西」**——比如 dryRun 模式：只定位不点击，并如实标出哪几步因为前置条件不满足而根本没检查过。

相关：[[snapchat-web-scripts]] · [[chrome-extension-e2e-automation]] · [[verification-discipline]]

---

## 自绘编辑器（Lexical / Draft.js / Snapchat 输入框）的录制与回放

2026-07-22 在 browser-replay 上踩到，两个坑都会导致**「每一步都成功、结果却是空的」**。

### 坑 1：录不到输入 —— `beforeinput` 被 preventDefault 后，浏览器不再派发 `input`

自绘编辑器的标准做法是在 `beforeinput` 里 `preventDefault()`，然后自己把内容写进内部 model 再渲染 DOM。默认插入被取消，**`input` 事件就不会产生**。只监听 `input` 的录制器完全瞎掉，导出的脚本里一条输入步骤都没有。

**修法**
- 监听 `beforeinput`：无论页面是否 preventDefault 都会派发，是唯一还能被外部观测到的输入信号。
- **值要延迟到结算时才读**：`beforeinput` 触发时内容还没落进 DOM，当场读只能读到旧值。
- 再加一层失焦兜底：值 ≠ 聚焦时的基线就补一条（覆盖表情面板、粘贴按钮这类纯 JS 塞值，连 `beforeinput` 都没有）。
- 用 WeakMap 记「已录入的值」做去重，顺带避免「点进点出、值没变也记一步」。

### 坑 2：回放写不进去 —— `execCommand('insertText')` 也不派发 `beforeinput`

编辑器的内容真源是内部 model，DOM 只是渲染结果。直接 `el.textContent = value` 它收不到，发送时读的还是空 model。

`document.execCommand('insertText', ...)` 看着更"原生"，**实测同样没用**：

```
execCommand('insertText') → ret:true, text:"hi", fired:null   // beforeinput 没派发
```

返回 true、DOM 也变了，但编辑器的监听器一次都没触发 —— 效果等同直写 textContent。

**修法：合成 `beforeinput` 喂给它**（编辑器就在那儿接管输入）

```js
const taken = !el.dispatchEvent(new InputEvent('beforeinput', {
  bubbles: true, cancelable: true, composed: true,
  inputType: 'insertText', data: value
}));
if (taken) { await nextFrame(); return; }   // 被吃掉 = 编辑器接管，绝不能再碰 DOM
el.textContent = value;                      // 普通 contenteditable 才走这条
el.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, data: value }));
```

`dispatchEvent` 返回 false 表示被 `preventDefault` → 编辑器已接管。此时**再去写 DOM 只会让两边状态打架**。

### ⭐ 方法论：断言要落在业务结果上

这次的失败形态是**回放日志全绿、消息一条没发出去**。步骤执行状态只能证明「选择器找得到元素」，证明不了「操作真的生效了」。测试必须断言业务结果（消息发出去了吗、表单值对不对），否则这类静默失败永远发现不了。

配套地，fixture 要**复刻编辑器的架构**而不只是加个 `contenteditable`：内部 model + 渲染 DOM + 发送时读 model。只写 `<div contenteditable>` 的 fixture 两个坑一个都复现不出来。
