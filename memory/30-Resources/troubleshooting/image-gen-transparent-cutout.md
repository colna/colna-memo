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
