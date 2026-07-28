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

---

## 记上下文属性：用排除法，不用白名单

抓元素周边上下文（祖先链快照等）时，第一版很自然会写成白名单 —— 只取 `data-testid` / `aria-label` / `role` / `name` 这类「像是有用」的属性。2026-07-22 实测这是错的。

**漏掉的恰恰是最需要的**：

| 属性 | 说明什么 |
|------|----------|
| `aria-expanded` / `aria-selected` / `aria-checked` | 当前是展开还是选中 —— 判断操作是否生效的唯一依据 |
| `tabindex` / `disabled` | 能不能交互 |
| 站点自定义 `data-*` | 往往是该站点唯一的语义标记，白名单不可能预先知道 |

白名单永远追不上各家站点的命名。**反过来做：默认全收，只排掉明确是噪声的** —— `class`（构建产物，改版即变）、`style`（渲染结果，逐帧都不同）。

两个容易写错的细节：

- **空值属性要记**。`disabled` / `hidden` 这类布尔属性的值就是空串，`if (value)` 会把它们整个跳过 —— 而「这个属性存在」本身就是全部信息。
- **id 仍要判稳**。React `useId` 的 `:r7:`、框架自增 id 每次渲染都变，全收会让两次录制没法比对。

> 这条直接决定了「点击后有没有生效」能不能验证 —— 选中态读不到，就只能返回 `null`。

---

## 四、「每步成功、结果是空的」的第二种形态(2026-07-24,Instagram DM)

同一个症状在 [[browser-replay]] 上第二次出现,这次是**三个独立缺陷叠加**,单修任何一个都不够。现场:Instagram DM(Lexical 编辑器)录了 10 步,回放第 6 步报「页面上找不到该元素。已尝试：」——**「已尝试」后面是空的**。

### 1. ⭐ 「保底候选」被自己的唯一性校验丢掉 → 候选归零

代码注释写着「结构路径一定能生成，作为保底」,实现却是 `push('structural', path, 40)` 而 `push` 默认 `needUnique=true` —— 不唯一直接丢。**注释承诺的保底，实现里根本没保**。

触发条件是 Instagram 那种每层单子节点的深嵌套:`nthOfType` 只在「同标签兄弟 >1」时才加 `:nth-of-type(n)`,于是每层退化成裸 `div`,12 层封顶后路径是 `div > div > … > div`,匹配上千个元素 → 判非唯一 → 丢弃 → **candidates 变成空数组**。

后果不只是失败,而是**失败得毫无信息**:`diagnose()` 遍历空数组,错误信息里「已尝试：」后面什么都没有,用户完全无从判断。

修法三层:
1. 常规路径不唯一时,退到**逐层强制 `:nth-child(n)`** 的加强版(封顶从 12 放宽到 30,一路走到锚点/根 —— 半截相对路径还不如全路径)。
2. 两条都不唯一**且此前一个候选都没攒到**时,无条件塞一条(降分到 20,UI 标「⚠ 易失效」)。宁可留一条可能选错的,也好过零候选。
3. 但第 2 条会**和焦点兜底打架**:有候选就会命中(错的)元素,兜底反而不触发 → 见下。

> 教训:**任何声称「保底」的分支,都要有一条测试断言它真的兜住了**。这条 bug 活了三个版本,因为所有测试用的 fixture 都是有标识的元素。

### 2. ⭐ 按键步骤应当退到 `document.activeElement`

键盘事件的目标**按定义就是当前焦点所在**,前一步的 focus/click 已经把焦点放对了。自绘编辑器的编辑区常常连一条唯一选择器都生成不出来(Instagram 的编辑区只有 `contenteditable` / `role=textbox` / `aria-placeholder`,**全都不在候选属性白名单里**)。

关键细节:**不是「找不到才退」**。只靠深层结构路径命中的元素很可能是同构的另一条链,按键打到错的元素比没打更糟。规则应是:强候选(测试属性 / id / name / aria,score ≥ 55)才信任,**弱命中一律让位给焦点元素**。

### 3. ⭐⭐ 输入事件可能压根收不到 —— 别把录制单押在 `beforeinput` / `input` 上

这次最要命的一条:这份录制里 **click / keydown 都收到了,输入事件一条没有**,于是 `pendingInput` 从未被设置,`flushPendingInput()` 什么都没吐出 —— **整段输入被静默丢掉**,导出的 JSON 里连一个 `input` 步骤都没有。即使定位全对,回放也只是在空输入框上按回车。

站点可以在 `window` 捕获阶段 `stopImmediatePropagation()` 掉输入事件(注册早于录制器就赢),自绘编辑器改由 keydown 维护内容。**keydown 这条通道实测始终是通的**,所以:

- **录制**:contenteditable 上打字时,keydown 也要 `markPendingInput`(值仍留到结算时读,不在事件里读)。
- **回放**:`typeIntoEditable` 在「合成 beforeinput 没人接管」后,先**逐键敲一遍**看内容有没有被页面自己写进去,再退到直写 `textContent`。直写只改到 DOM 这层投影,发送时读的还是空 model —— 就是第一次那个坑。
- **待结算目标要归一到「编辑宿主」**(带 contenteditable 的那个元素),不能拿深层的 `p`:Lexical 每次输入都重建内部节点,结算时旧节点已脱离文档,`isContentEditable` 变 false、`canRecordValue` 判否,同样静默丢掉。

> 通用原则:**同一件事有多条观测通道时,不要只订一条**。哪条被掐了是站点说了算,不是你说了算。

---

## 五、排查扩展/注入类问题时,先证明「跑的是不是新代码」(2026-07-24)

在 [[browser-replay]] 上连续两轮修改「看起来完全没生效」,最后发现是**页面里跑的一直是旧 content script**。为此白排查了两次,期间还据错误现象下了一个错误结论(以为站点把 keydown 也吃掉了)。

### 1. ⭐ MV3 的注入哨兵不能用布尔量

```js
if (window.__BR_CONTENT__) return;     // ← 陷阱
window.__BR_CONTENT__ = true;
```

扩展重新加载后,老实例仍留在**已打开的**标签页里 —— 它的 `chrome.*` 上下文已失效,但 `window` 上的标记还在。此时覆盖注入(`chrome.scripting.executeScript`)新代码,新代码撞上这个哨兵**一行都不跑就 return**,页面就此永远停在旧版本。

改成版本号哨兵:

```js
const VERSION = chrome.runtime.getManifest().version;
if (window.__BR_CONTENT__ === VERSION) return;   // 版本不同就让新代码接管
window.__BR_CONTENT__ = VERSION;
```

配套:`ensureInjected` 不能只判断「ping 得通吗」,要**比对版本**——旧实例照样应答 ping。

### 2. ⭐⭐ 版本标记要盖在「产出物」上,不是「读取时」

第一版我把版本加进导出 JSON,以为这样就能确认代码是否生效。**它证明不了**:那是 background 在**导出那一刻**读 manifest 得到的,只说明扩展是新的,不说明**录制当时页面里那个 content script** 是哪一版。两者经常脱节。

正确做法:**由真正干活的那一方在干活的那一刻盖章**,随产出物一起存。

```
extVersion      导出时的扩展版本      ← 证明不了什么
contentVersion  录制时的脚本版本      ← 排查先看它
```

> 通用原则:**排查「改了没生效」之前,先花五分钟拿到一个能证明代码版本的信号**。没有它,后面每一轮分析都建立在「新代码在跑」这个未经验证的前提上 —— 而这个前提一旦为假,你会顺着完全正确的推理走向完全错误的结论。

### 3. ⭐ 「找不到」和「找错了」不是同一量级的失败

同一轮踩到的另一条。给弱候选(结构路径)加校验后,我又多写了一层「都不像就退回不校验的第一个匹配」,理由是「校验只是排序偏好,不该把有点东西变成什么都没有」。

**这个取舍是错的**。实测代价:Instagram 的发送按钮在输入框为空时原地变成麦克风,位置一模一样。语义候选 `[aria-label="发送"]` 匹配不上后退到结构路径,按位置命中麦克风 —— **「回放」变成了「开始录音」**。

- 找不到 → 这一步停下,报错,人能看见
- 找错了 → **执行了另一个动作**,而且每步都显示成功

给任何兜底加「宁可命中点什么」的退路前,先问:命中错的会发生什么?

## 2026-07-27 IG toProfessionalAccount 踩坑(app-ins-scripts)

- **`utils.simulateClick` 会一次触发 2~3 次点击**(合成事件链 + 原生 `el.click()` + 直接调 React `onClick`)。IG/React「点一次进一步」的向导按钮用它会**一次跳过好几步**(真机现象:脚本冲到成功页跳过类别,手动却一步步)。→ 向导「下一步/完成/继续」用**单次 `el.click()`**(`clickOnce`);radio 选择保持 simulateClick(选中幂等无害)。
- **判「到没到某页」不能看子元素数**:IG 类别选项**懒加载**,radiogroup 先挂空壳再填。判到页看**容器存在**,再轮询等选项渲染。
- **向导主按钮点前必查 `disabled`**:选够条件才解禁,盲点无效。
- **radio 的 `aria-label`/`value` 常是数字 id**(类别 2201=产品/服务),可见文案在同行 span → 匹配要**兼顾文案与 id**。
- **新增 method 两步**:①`src/instagram/methods/<name>.js` 挂 `window.SocialProxy.<name>` ②`scripts/build.mjs` GROUPS.automation 加 `m:<name>`。dispatch 是 `SP[action]` 泛化,无白名单。
- **文案进共享 `utils.LABELS`(中/英/西)**,用 `matchLabelLoose`,别在 method 内硬编码数组。
- **判断容器跑没跑最新代码看日志格式/新字段**:改完 dist 型产物必须 `node scripts/build.mjs` 重建 + **重新注入** bundle;光看行为易误判成逻辑 bug,其实是 dist 没更新。
