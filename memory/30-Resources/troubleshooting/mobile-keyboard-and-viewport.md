---
title: 移动端 on-screen keyboard 与 viewport 处理踩坑合集
date: 2026-07-13
tags: [troubleshooting, pwa, frontend, mobile, viewport, keyboard, react]
---

# 移动端「on-screen keyboard + viewport」踩坑合集

来源:sitin-next `app-pwa` Chat 页键盘弹起把底部 TabBar 一起顶起(PR [#597](https://github.com/presence-io/sitin-next/pull/597),2026-07-13,`packages/app-pwa/src/components/TabLayout.tsx`)。

主线两大共性坑:
1. `<meta name="viewport" ... interactive-widget=…>` 的**三档语义**决定键盘怎么"吃"视口 —— 靠 CSS `h-full`/`100vh` 的固定布局在不同档下表现截然不同。
2. **输入框 `:focus` 状态 ≠ 键盘可见**。用户可以在 focus 状态下手动收起键盘,反过来键盘可能在自动完成/预输入时短暂弹起而无 focus 变化。凡是"跟键盘对齐 UI"的场景都不能靠 `:focus`。

---

## 1. `interactive-widget` 三档语义:layout viewport 会不会跟着缩

`<meta name="viewport" content="… interactive-widget=X">`(Chromium spec):

| 值 | Layout viewport | Visual viewport | 典型现象 |
|---|---|---|---|
| `resizes-content`(Chrome for Android 默认) | **缩** | **缩** | `100vh` / `h-full` 容器整体缩短,固定在底部的元素被顶到键盘上沿(Chat 页 TabBar 就是这样被顶起) |
| `resizes-visual` | 不缩 | 缩 | 布局稳定,浏览器自动 scroll 让 focus 元素露出 |
| `overlays-content` | 不缩 | 不缩 | 键盘直接覆盖底部,需自己用 `env(keyboard-inset-height, 0)` 或 `visualViewport` 顶起输入栏,否则输入栏被盖住 |

**结论**:`resizes-content` 是"锅在浏览器帮你让位"的默认,但代价是任何 `flex-1 + h-full + 底部元素` 的布局都会**整体被键盘顶起**,包含底部 nav。

## 2. Layout viewport 和 visual viewport 一起缩时,不能靠它俩相比判定键盘

- 直觉:`visualViewport.height < window.innerHeight` → 键盘弹起。
- 现实:**只在 `resizes-visual` 下成立**。`resizes-content` 下两者同步缩,ratio 恒 ≈ 1,这条判据失效。
- 通杀写法:**只信 `visualViewport.height` 自身相对 baseline(最大值)的下跌**,不跨 layout / visual 比。

## 3. 用 `:has(input:focus)` 隐藏 TabBar,漏 "focus 保留但键盘收起" 场景

**v1 修法**(纯 CSS,PR #597 commit `4b33ddfc0`):

```tsx
<div className="group/tab-layout ...">
  ...
  <div className="group-has-[input:focus,textarea:focus,[contenteditable]:focus]/tab-layout:hidden">
    <TabBar />
  </div>
</div>
```

**为什么翻车**:
- 用户点输入框 → focus → keyboard 弹 → TabBar 藏 ✅
- 用户下拉键盘 / Android 系统返回收键盘 → **input 常不 blur**(浏览器实现差异,Chrome for Android 大多保留 focus,iOS Safari 有时 blur)→ `:focus` 仍匹配 → TabBar 一直藏 → **键盘位置空一大块** ❌

**根因**:`:focus` 反映的是"当前焦点元素",不是"键盘是否可见"。这两个状态在移动端是弱耦合的。

## 4. baseline-max 阈值法:与 focus 解耦地检测键盘可见

**v2 修法**(commit `84874c8e3`):

```tsx
function useKeyboardOpen(): boolean {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const vv = window.visualViewport;
    if (!vv) return;
    let baseline = vv.height;
    const onResize = () => {
      if (vv.height > baseline) baseline = vv.height; // 只涨不跌
      setOpen(baseline - vv.height > 150);              // 掉 >150px 才判定键盘
    };
    vv.addEventListener("resize", onResize);
    return () => vv.removeEventListener("resize", onResize);
  }, []);
  return open;
}
```

**几个关键设计**:
- **baseline 只涨不跌**:横竖屏切换、URL bar 展开都会让 viewport 变大,及时抬高 baseline;键盘弹起让 viewport 变小,不动 baseline。
- **150px 阈值**:Chrome for Android URL bar 展开/收起约 60px,阈值 150px 挡掉这类误报。iOS Safari 键盘约 260~300px,Android 300~400px,150 是安全下限。
- **不看 focus 状态**:v1 因为看 focus 而漏掉"focus 保留 + 键盘收起"case,v2 只看视口大小,健壮。

## 5. Feature detection:`window.visualViewport` 可能是 undefined

- iOS < 13 / 极老 Android WebView 没有 `visualViewport`。
- **写法**:`const vv = window.visualViewport; if (!vv) return;`,不加 `!vv` 会 SSR 崩(现代 React SSR / hydration 时也可能)。
- 目标客群若能保证 modern browser,直接 return / 不管;否则退化为"永远显示 TabBar",UX 退化但不会崩。

## 6. React 状态 vs CSS-only 的选型

- **可以纯 CSS 就纯 CSS**:少一个 hook 少一次 rerender。适用于「事件源和 UI 变化的映射永远是恒等」的场景。
- **一旦事件源(键盘可见)和 CSS 能感知的信号(`:focus`)不等价,回退到 JS state**。硬套 CSS 会积累"感觉对但边角 case 挂"的技术债。

## 7. pre-push hook 挂在无关包 → 用 `--no-verify` 前要拉基线对照

- `sitin-next` pre-push 跑全库 `build+test`。`business-minerva-*` 有既有编译失败(esbuild 崩),会拖住所有 branch 的 push。
- **判定"是不是我引入的"**:切回 base branch(`feature/sitin4.0`)复现同一条编译 —— 若 base 也挂,就是既有 bug,`--no-verify` 是合规操作。
- CLAUDE.md 只显式禁 `git commit --no-verify`(强制本地 lint),对 `git push --no-verify` 未禁 —— 但仍需征得用户同意。

---

## 附:相关阅读

- Chromium docs: [Interactive Viewport](https://developer.chrome.com/blog/viewport-resize-behavior)
- W3C VisualViewport API: <https://drafts.csswg.org/cssom-view/#visualviewport>
- 移动端布局的 `env(keyboard-inset-height, 0)` 备用方案(仅 `overlays-content` 下有意义)。

## 5. APK WebView 里键盘弹起底栏仍被顶起(web 正常)——两套机制打架

来源:GraceChat-Earn-Android(`p/ljb/sitin4`)+ sitin-next app-pwa(2026-07-14)。web 端 §4 的 baseline-max 修复有效,但 APK WebView 里底部 TabBar 仍跟键盘弹起。

**根因:APK 下有两条键盘链,一失效一帮倒忙**
1. **navbar 隐藏靠 `visualViewport`(前端)在 APK 失效**:安卓 `PWAWebViewFragment` 用 `enableEdgeToEdge()`,`setOnApplyWindowInsetsListener` **只消费 `systemBars()` 不消费 `ime()`** → 键盘弹起时安卓**不 resize WebView 窗口**,只通过 bridge 通知 → `visualViewport.height` 不变 → `useKeyboardOpen` 恒 false → TabBar 不隐藏。
2. **安卓 `notifyKeyboardChanged` bridge 把 body 顶上去(只 APK 有)**:安卓 `PWAWebViewFragment.kt` 的 `KeyboardChangedListener`(Android30+ 走 `onKeyboardAnimStart/Progress/End`,<30 走 `onKeyboardHeightChanged1`)`callHandler("notifyKeyboardChanged", {show,height})`;前端 `bridge.ts` handler 收到做 `document.body.style.paddingBottom = height` → 把 `h-full` 流式布局的整个 body(含 flex 底部 TabBar)整体上推一个键盘高度。
- 叠加 = TabBar 没隐藏 + 被 body padding 顶起 = 底栏跟键盘弹起。
- **web 正常**:无 notifyKeyboardChanged(不推 body)+ `interactive-widget=resizes-content` 让 visualViewport 真缩 → useKeyboardOpen 生效隐藏 TabBar。

**判据速记**:同一份前端,web 正常、APK 底栏被顶 → 大概率是「APK 有 native 键盘 bridge 主动改布局(body padding / 顶起)」且「visualViewport 在 APK 不 resize 导致纯前端键盘检测失效」两件事叠加。先查 native 有没有 `notifyKeyboardChanged` 之类 bridge + WindowInsets 是否消费了 `ime()`。

**修法**
- A(前端,推荐):`useKeyboardOpen` 在 APK 改用 `notifyKeyboardChanged` 的 show 状态(暴露成 store 给 TabLayout 订阅),web 仍用 visualViewport。navbar 隐藏后 body padding 只推输入栏。不动安卓。
- B(安卓):WindowInsets 加消费 `ime()` 让 WebView 真 resize(对齐浏览器),去掉前端 body padding hack;更正统但影响全局布局、回归面大。

## 8. APK overlay 模式:输入框上抬 `--kb-height` 的完整方案 + native 行为踩坑

来源:sitin-next app-pwa,PR [#615](https://github.com/presence-io/sitin-next/pull/615)(2026-07-15,`utils/keyboardLift.ts`)。接 §5——navbar 用 `KeyboardAwareTabBar`(APK 恒不隐藏、web 隐藏)后,APK 键盘 overlay(nav 不动、只抬聊天输入框)的最终实现。lift = 输入区 `translate3d(0, calc(-1*max(0px, var(--kb-height) - var(--nav-height))), 0)`。

**native `notifyKeyboardChanged` 的 4 个行为特性(全是坑的根源,靠页面日志浮层在真机抓出来)**
1. **逐帧上报**:`show=true` 时每帧推 `height`(键盘动画跟随),不是一次给终值。
2. **少报工具栏**:同一个带工具栏的键盘,有时报含工具栏的完整高度(如 351),有时报不含的(320,差 ~31px)——**全新弹出就可能少报** → 输入框贴到偏低位、被 IME 工具栏盖住。不是切后台才有。
3. **收起 = 平滑递减 + 末段抖动**:收起时 `show` 仍 `true`,`height` 逐帧平滑降到 0,末段在 0/1/2 之间**抖动/持平**,最后一帧才 `show=false`。不是单帧骤降、也不是单调递减。
4. **回前台会重发**:切后台→回前台 native 会重新逐帧发 `notifyKeyboardChanged`(`show=true h=偏小值`)→ handler 自己能覆盖,**不需要 `visibilitychange` 兜底**(早期加的 `readActiveInputKbHeight`+visibilitychange 是 dead code,已删)。

**演进中翻车的判断(每个都被真机日志推翻)**
- **逐帧写实报值 + floor 到历史最大** → 位置对但**不跟手**:任何低于 max 的帧(二次弹出的上升、收起的下降)都被强行抬到顶。
- **「单帧骤降 > N」判收起** → native 是平滑递减(每帧跌几 px),**永不触发**。
- **「方向下降(h < prevH)」判收起** → 末段 h 抖动/持平(1==1、0==0)时 `<` 为 false → 翻回"打开"→ target 回到完整高度 → **落到底又往上弹一次**。
- **完整高度靠逐帧累积** → 首次弹出 target 逐帧变大、CSS transition 追不上 → **上升慢、非一步到位**。
- **缓存只增、不分场景** → 横屏键盘高度 / 表情面板高度 / 异常大值把 `lastKbFullHeight` 永久污染。

**最终方案(纯 CSS 驱动,不逐帧追 native)**
- **一次写目标**:`show` → 目标 = `lastKbFullHeight - GAP`;`!show`/收起 → 0。CSS transform `transition 0.2s cubic-bezier(0.2,0,0,1)` 负责上升下降的缓动,一步到位。
- **完整高度 = 历史最大 + 持久化 localStorage**:冷启动首次即读缓存一步到位(否则逐帧累积被 CSS 拖慢)。
- **收起锁存(latch)**:高度从「近满峰值」回落 > `KB_RETRACT_DELTA(45)` 即 latch `closing`→target 0,**保持到 `show=false`**;末段抖动不再翻转(修「落到底又弹回」)。高度回到近满(重新打开/回前台)才解锁。
- **缓存防污染 = 按方向分 key + sanity 区间**:`kb_full_height_px_portrait/_landscape` 两个 key(旋转时切换+重置 latch/peak);只缓存落在 `innerHeight` 的 **20%~70%** 的高度(挡表情面板/回前台中间值/异常值)。
- **一致性**:Chat / TutorialChat / `/dev/chat-input-bar` 三处输入区共用同一个导出常量 `KEYBOARD_LIFT_STYLE`,永不漂移。
- **模块化**:全部逻辑在 `utils/keyboardLift.ts`(对外仅 `handleNativeKeyboardChanged(data)` + `KEYBOARD_LIFT_STYLE`),`bridge.ts` 只做端能力挂载(handler 一行转发)。

**方法论**:APK WebView 无 devtools、on-device vConsole 也不便 → 在 `bridge`/键盘模块里挂一个 `position:fixed` 的**页面日志浮层**(绿字、`pointer-events:none`、只记变化事件),真机截图回传即可看清 native 逐帧时序。定位完再单独 commit 删除。这套是本次一切根因定位的关键工具。
