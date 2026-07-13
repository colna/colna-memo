---
title: 从印刷/屏幕色卡照片取色 — 避文字污染 + 中位数采样
date: 2026-07-11
tags: [color, palette, sampling, python, PIL]
---

# 场景

给一张色卡照片(印刷或屏幕拍摄),要为每个色号提取近似 RGB/HEX,做数字色板。
色号文字通常**居中白字**印在色块上,直接窗口平均一定被文字污染。

## 三次迭代教训(2.6MM 小豆 221 色马克笔实测)

**v1 直接平均 → 偏灰绿**

窗口 12×12 取每格中心 → A01(浅黄)得 `#a19e84`,肉眼看是暗灰绿。原因:白字压低了饱和度、拉高了灰度。

**v2 阈值过滤白像素 → 有改善仍偏灰**

过滤 `min(R,G,B) > 235` 的近白像素后取平均 → A01 得 `#cac7ac`。抗字符干扰了,但字符边缘的半透明像素仍拉低饱和度。

**v3 ✅ 三招组合**

1. **采样点 y 方向下偏 8 像素**避开居中色号文字
2. 过滤 `min(R,G,B) > 230` 的白像素
3. **每通道取中位数**(不是平均)

A01 得 `#f1edce`,合理。

## 复用清单

- **必用中位数,别用平均**:中位数对残留白字/边缘半透明像素稳健。
- **采样点偏离文字位置**:色卡的文字位置(左上角编号 / 居中色号 / 右下角厂标)决定偏移方向,别在文字上采样。
- **白像素阈值 230~235**:太严会漏掉亮色本身(浅黄本来就接近白),太松挡不住字符。经验值 230。
- **窗口尺寸 16×16 起**:太小噪声大,太大跨格,16 是甜点。
- **色号定位靠拟合,不靠猜**:用最长列拟合 `step = (y_last - y_first) / (n - 1)`,别硬编码格间距。

## 参数模板(Python + PIL)

```python
from PIL import Image
import statistics

im = Image.open(path).convert("RGB")
px = im.load()

def sample(cx, cy, win=16, y_offset=8, white_thresh=230):
    r_list, g_list, b_list = [], [], []
    for dy in range(-win//2, win//2):
        for dx in range(-win//2, win//2):
            r, g, b = px[cx + dx, cy + y_offset + dy]
            if min(r, g, b) > white_thresh:
                continue
            r_list.append(r); g_list.append(g); b_list.append(b)
    if not r_list:
        return None
    return (
        int(statistics.median(r_list)),
        int(statistics.median(g_list)),
        int(statistics.median(b_list)),
    )
```

## 精度声明

采样自屏幕显示的色卡照片,受**屏幕色域、印刷偏色、图片压缩**影响,与实物笔色可能有 5-15% 偏差。用于原型/草图选色够用;严格配色以厂家官方色值或实物为准。

## 相关

- 产物示例:[[../marker-2.6mm-221colors]]
- 消费方:`.claude/skills/marker-pixelize/`(把 221 色 palette JSON 内嵌到 skill)
