# 透明底素材：什么时候不能用 flood fill 抠图

来源：2026-09-10 出 Luka「探头 + 气泡」透明底 PNG。

## 判断用哪条路

| 情形 | 用什么 |
|---|---|
| 文生图、无参考图 | `gen_image.py --transparent`，直接出 RGBA |
| 图生图（`--ref`），主体是实心色块、边缘干净 | 白底 + `cutout_bg.py`（flood fill） |
| **主体含白色元素**（白气泡、白衣服、白高光） | **不能用白底**，见下 |
| **主体边缘是毛发／绒毛／细丝** | **不能用 flood fill**，见下 |

## 坑一：主体里有白色 → 白底会被打穿

`cutout_bg.py` 从画布四边 flood fill。白气泡和白底同色，只要通过背景连通到画布边缘，
整个气泡都会被判成背景。

**改用彩色幕**，挑一个主体身上绝对不会出现的颜色。暖色角色用**品红 `#FF00FF`**：

```
BACKGROUND: PURE FLAT MAGENTA #FF00FF, absolutely uniform across the whole frame, edge to edge.
NO glow, NO halo, NO vignette, NO gradient, NO coloured rim light.
NO drop shadow, NO contact shadow, NO ground plane, NO reflection.
```

## 坑二：毛发边缘 → flood fill 进不去，留下一圈不透明幕布色

毛丝之间的幕布被前景包住、**连不到画布边缘**，flood fill 到不了，于是留成
alpha=255 的品红像素。叠到深色底上就是一圈紫描边。
`cutout_bg.py --bg 255,0,255` 治不了这个——它的机制就是连通域。

**改用 chroma key**（逐像素判据，不管连通性）：

```python
# 品红的特征：R 和 B 都远高于 G。对暖棕/奶油白主体是唯一的判据
# 实测 spill：品红 +255、棕毛 −40、奶油白 −11、粉鼻子 −9、白气泡 −5
spill = np.minimum(R, B) - G
alpha = 1.0 - np.clip(spill / 110.0, 0.0, 1.0)
alpha[alpha < 0.06] = 0                       # 太淡的边全丢，免得放大噪声

# 去色（spill suppression）：C = a·F + (1-a)·BG，反解 F
F = (C - (1 - a) * BG) / max(a, 1e-6)
```

换幕布色就换判据：绿幕用 `G - max(R,B)`，蓝幕用 `B - max(R,G)`。
**先拿主体上最接近幕布色的那个部位算一遍 spill**（这次是粉鼻子），确认它是负的再动手。

## 坑三：纯黑底 + 毛发 → flood fill 留下黑边，用软 alpha + unpremultiply 补

2026-09-15 Luka Calls 空态把小组件那张黑底 JPEG（`widght4.jpeg`）用到奶油底页面上时发现：
早先 flood fill 出的透明 PNG（`widgets/states/state-worried.png`）在浅底上有一圈**黑色噪点** ——
抗锯齿像素是「毛发↔黑」的混色，flood fill 只做二值判断，把这些半黑像素留成了 alpha=255。

黑底的好处是**混合公式已知**：`观测 C = α·F`（F 是真实颜色），于是

- 软 alpha：把硬 mask 模糊一下就是抗锯齿边缘（`-blur 0x0.9 -level 5%,95%`）；
- 去色（unpremultiply）：`F = C / max(α, ε)`，反解回毛发本色。

```bash
magick hard.png -blur 0x0.9 -level 5%,95% soft.png        # hard = 原 PNG 的 alpha 通道
magick "$SRC" soft.png -fx "u/max(v,0.004)" -clamp -strip unpre.png   # u=源图, v=alpha
magick unpre.png soft.png -alpha off -compose CopyOpacity -composite out.png
```

- 不要用 `-transparent black`：硬阈值，黑边照样留。也别靠 `-morphology Erode` 缩轮廓 —— 会削掉胡须/绒毛。
- 验收：抽几个内部像素比对源图颜色（应完全一致），再叠到奶油/深色两个底上看边缘。

## 验收

把结果分别叠到深色、奶油、饱和蓝三个底上看边缘，**不要只看棋盘格预览**——
棋盘格是浅色的，幕布残留在上面几乎看不出来。

```python
c = Image.new("RGBA", im.size, (26,20,16,255)); c.alpha_composite(im)
```

再补一条数值复检，别只靠眼睛：

```python
vis = A > 20
bad = ((np.minimum(r,b) - g) > 25) & vis      # 可见像素里还有多少偏幕布色
```

## 附：多轮改图时，参考图要先落到自己的目录

飞书发来的图落在 `/var/folders/.../metabot-downloads-max/`，**这个目录会被清空**
（实测几分钟内就没了），第二轮 `--ref` 会直接报 `参考图不存在`。

拿到参考图第一件事：

```bash
cp "$FEISHU_IMG" "$SCRATCHPAD/ref-<说明>.png"
```

之后所有轮次都引用 scratchpad 里的副本。

## 附二：幕布是一块圆形留白，要把别的图层嵌进去

2026-09-20 Luka Quick Pick 的仓鼠画框（`mole-frame.webp`）：image2 出的「仓鼠抱着一个圆」，
圆里是绿幕、外面是白底（生成时就带 alpha）。要把真人头像嵌进那个圆里，做法不是「抠干净再拼」，
而是**只抠那个圆**、把它留成透明的洞，头像在 RN 侧垫在画框下面：

1. **外背景**：图本身已带 alpha 就沿用；白底不透明时才需要 flood fill / 逐像素判据。
2. **抠圆用色键、不要用几何裁圆**：前景（爪子）压在圆上，几何裁圆会把爪子一起切掉。
   判据 `spill = min(G-R, G-B)`，`alpha *= 1 - clip((spill-8)/30)`，`spill > 23` 直接清零。
3. **量圆**：抠之前先用 `(G-R>25)&(G-B>25)` 取绿像素 bbox 拟合圆心与半径（本次 1024 图：
   圆心 493/627、r 317.5）。这三个数就是代码里头像盒子的 `left / top / 直径`，按素材比例换算成 pt。
4. **嵌入层要「压进」环带底下**：头像直径取圆的 1.01 倍左右（本次 71.2 → 72），边压在环带
   下面，圆与环之间才不留底色缝；超过环外缘又会从环外露出来。所以还要量环带厚度（本次
   11px），约束是 `r_头像 ∈ (r_圆 + AA 3px, r_圆 + 环厚)`。
5. **缩放走预乘 → LANCZOS → 反预乘**，否则透明区的黑 RGB 会在边缘渗出黑边；透明区的 RGB
   先用「预乘高斯模糊 ÷ 模糊后的 alpha」外推一层补上。
6. **收尾复检**：`(min(G-R,G-B) > 12) & (alpha > 40)` 必须为 0 —— 重点是环外缘那圈抗锯齿
   （本次原图 13 个，缩放又在环外带出 13 个，两处都要清）。
