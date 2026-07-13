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
