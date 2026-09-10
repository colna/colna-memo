# 按设计稿写设备内 UI 的三个坑

来源：2026-09-10 还原 profile/[id] 页 HTML demo。

## 1. 从 mockup 截图上裁素材会把 UI 一起裁进去

设计稿里照片角上本来就画着返回键、更多键、小头像。裁下来当 `<img>` 素材，再用 CSS 叠一层真按钮，
结果就是**同一个按钮出现两次**（一个偏移几十 px、颜色略淡）。

排查特征：重影只出现在「图片上的绝对定位元素」，位置偏移固定，缩放截图能看清是两个实体不是渲染毛边。
别怀疑 headless 合成 bug、别去调 `backdrop-filter`。

修法：用 `image-gen --ref <mockup>` 重出一张**纯照片**，prompt 里逐项写死排除项：

```
MUST NOT appear anywhere: phone frame, bezel, screen, status bar, app UI, buttons,
circular badges, small circular avatar overlay, arrows, dots, cards, rounded corners,
borders, letters, words, watermark.
```

## 2. `box-sizing: border-box` 下机身宽度要含边框

```css
*{box-sizing:border-box}
.device{width:402px; padding:11px}   /* ✗ 屏幕只剩 380 */
.device{width:424px; height:896px}   /* ✓ 402×874pt + 11px 边框×2 */
```

窄 22px 看着不明显，但会连锁导致「标签排不下 3 个 / 文案多断一行」，
很容易误判成字号问题，然后一路把字号调小、越调越不像稿。
**先量卡片实际宽度再动字号。**

## 3. AI 生成的 mockup 屏幕比例比真机长

gpt-image-2 出的 iPhone mockup，折算下来屏幕是 402×932，而 iPhone 17 是 402×874。
照抄稿上的元素尺寸会**溢出约 58px**，底部按钮压住内容。
必须挑一处压缩（一般压 hero 的 `aspect-ratio`），并在交付时说明这是适配真机的取舍。

## headless 里怎么量元素宽度

没法开 DevTools 时，把测量结果塞进 `document.title` 再 `--dump-dom` 捞出来：

```bash
# 页面里：setTimeout(()=>{document.title=JSON.stringify({w:el.getBoundingClientRect().width})},100)
chrome --headless=new --virtual-time-budget=800 --dump-dom "file://page.html" \
  | grep -o '<title>[^<]*</title>'
```

截图用 `--headless=new --run-all-compositor-stages-before-draw`，比 `--virtual-time-budget` 稳。

## 4. 让一个元素「从某块底色后面探出」

子元素永远画在父元素自己的 `background` 之上，所以把吉祥物放进卡片里只会「站在卡上」，
拿不到「从卡后探头」的效果。

```css
.band{ position:relative; isolation:isolate; }        /* 免得 z-index 泄到外面 */
.band::before{ content:""; position:absolute; inset:0; z-index:1;
               background:var(--band); border-radius:18px; }   /* 底色单独画一层 */
.band .pet { position:absolute; z-index:0; bottom:0; }          /* 探出的那位在底色下面 */
.band p    { position:relative; z-index:2; }                    /* 文字在最上面 */
```

调「露多少」有两个数联动，别只调一个：
`pet` 的高度、以及条自身的高度（文案换行会把条撑高，露出的比例跟着变）。
文案能不能一行放下，直接决定探出来的是「整个头」还是「两只耳朵」。

## 5. 照 AI 出的 UI 稿定尺寸：先找基准元素，别按像素等比换算

AI 生成的 UI 稿**内部比例是不自洽的**。实测同一张 1024×1536 的稿子（对应 402pt 宽屏）：

| 元素 | 图上像素 | 等比折算 | 代码实际 | 对得上吗 |
|---|---|---|---|---|
| 头像直径 | 190 | 74.6pt | 76pt | ✅ |
| 名字字号 | — | 15.6pt | 19pt | ❌ |
| 按钮高度 | 55 | 21.6pt | 44pt | ❌（21pt 根本点不动） |

所以照搬像素会得出「按钮 21pt 高」这种不能用的结论。

**做法**：挑一个两边已经对齐的元素当基准（上表里是头像），改用**比值**定尺寸：

```
稿子：按钮高 / 头像 = 55 / 190 = 29%
真机：按钮高 / 头像 = 44 / 76  = 58%     ← 难怪显得重
取 45% → 76 × 0.45 ≈ 34pt
```

**收到 44pt 以下就要补 `hitSlop`**，把可点区域还回去：

```tsx
const HIT_SLOP = { top: 6, bottom: 6, left: 8, right: 8 };  // 34 + 6 + 6 = 46pt
<Pressable hitSlop={HIT_SLOP} ... />
```

看起来小、点起来不小。
