---
title: 不上真机判断装饰件压没压到素材 —— 抽帧 + 逐带轮廓 + 按 pt 合成
date: 2026-09-11
tags: [troubleshooting, react-native, ffmpeg, imagemagick, sitin-rn, luka, ui]
---

# 装饰件压没压到仓鼠:先把坐标量出来,再改数

**场景**(2026-09-11,Luka `my-feed-pet.tsx`):实机截图反馈「爱心压到仓鼠了,爱心往右一点」。
「一点」是多少 pt 要拿数说话,而真机/模拟器迭代一次要切屏、截图、加载,慢。

**三步法**,全程离线、可复跑、约 10 秒:

## ① 抽一帧看清素材的实际占位

```bash
ffmpeg -y -v error -i stay-all.mov -frames:v 1 -pix_fmt rgba /tmp/pet-frame.png
magick identify -format '%wx%h\n' /tmp/pet-frame.png          # 320x320 画布
magick /tmp/pet-frame.png -trim -format 'trim=%wx%h%O\n' info:  # 内容并集包围盒 271x316+23+4
```

⚠️ **本机 ffmpeg 7.1.1 解不出 HEVC-with-alpha**(实测 `transparent_ratio=0.000`),抽出来的是
**不透明黑底**。这反而是好事:黑 = 透明区,**拿它当蒙版/轮廓正好**;但别拿它判「素材有没有透明通道」
(那条另见 [带 alpha 的 MOV 压缩](mov-alpha-compression.md),9.0.1 起能解)。

## ② 逐横带裁一次,得到「素材右沿随高度怎么变」

```bash
for Y in 0 20 40 60 80 100 120; do
  printf 'y=%3s  ' $Y
  magick pet-frame.png -crop 320x20+0+$Y +repage -trim -format '%wx%h%X%Y\n' info:
done
```

每带报出「宽×高 + 左上偏移」,右沿 = 偏移 x + 宽。本次换算(320 画布 → 60pt 方框 = ×0.1875):

| 带 (px) | 右沿 (pt) | 是什么 |
| --- | --- | --- |
| 0–20 | 43.5 | 右耳顶 |
| 20–60 | 45.4 | 耳根 |
| 60–80 | 48.4 | 腮 |
| 80–100 | 51.4 | 脸侧 |
| 100–140 | 52.7 → 54.4 | 身子最宽 55.1 |

**这一步是关键**:只看「素材总宽」会以为右侧还有富余,剖面才看得出**耳朵在最上面反而最窄**、
而装饰件通常也挂在最上面 —— 竖向位置差 10pt,可用宽度就差了 9pt。

## ③ 按代码里的 pt 把「素材 + 装饰件 + 相邻元素边界」合成出来比候选

单位取 pt × 4(一点一点量像素太累,4× 足够看清压没压)。

```bash
FONT=/Users/colna/WORK/sitin-rn/node_modules/@expo/vector-icons/build/vendor/react-native-vector-icons/Fonts/Ionicons.ttf
magick -background none -font $FONT -fill '#F2B5BC' -pointsize 40 label:$'\uf36a' h-s.png   # 10pt 心
magick -background none -font $FONT -fill '#F2B5BC' -pointsize 52 label:$'\uf36a' h-b.png   # 13pt 心

for L in 35 40 44 46 48 52; do                     # 候选左边距(代码里是 PET_SIZE - N)
  XS=$((L*4)); XB=$(((L+12)*4))
  magick -size 300x250 canvas:'#FDF9EE' \
    \( pet-frame.png -resize 240x240 \) -geometry +0+0 -composite \
    h-s.png -geometry +$XS+36 -composite h-b.png -geometry +$XB+0 -composite \
    -stroke '#FF00FF' -strokewidth 1 -fill none -draw 'line 240,0 240,250 line 288,0 288,250' \
    -stroke none -fill '#F7CFD5' -draw 'roundrectangle 288,70 300,230 12,12' \
    -pointsize 20 -fill '#333333' cand-$L.png
done
magick montage cand-*.png -tile 1x -geometry +0+4 -background '#DDDDDD' cands.png
```

- 粉色圆角块 = 右侧那颗气泡在合成里占的位置(它从 `PET_SIZE + GAP` 起),洋红线标出
  **素材方框右沿**与**气泡左沿**两条边界。
- 心形不是手画的:用**真字形**。`Icon` 的语义名到字形的映射在
  `apps/luka/src/components/ui/icon.tsx`(`heart-filled` → Ionicons `heart`),码点从
  `.../react-native-vector-icons/glyphmaps/Ionicons.json` 查(heart = `62314` = U+F36A)。

## 合成要能代表真机,得先核对三件事

1. **盒子与素材同构**:`PetAnimation.ios.tsx` 是 `contentFit="contain"` + 方形素材 + 方形 box
   (`size` 同宽高)→ 等比填满,可直接按「画布 → box」线性换算。`cover` 或非方形就要重算。
2. **装饰件用的是同一个字形**,别拿近似的 SVG 顶替 —— 形状差一点,肉眼判断就会被带偏。
3. **绝对定位不裁剪**:RN 默认 `overflow: visible`,心可以溢出方框、也可以压到邻居上。
   合成里照样画,不要假设被裁。

## 这次的数

耳沿 45 上下、头往下变宽到 52.7、气泡 72 → 心放 `PET_SIZE - 14`(46):贴着耳朵飘,停在气泡前 2pt。
上一版 `PET_SIZE - 25`(35)那颗小心整颗落在耳朵与腮上。

**最后一步仍然要真机/模拟器看一眼** —— 合成能证明「不重叠」,证明不了「好看」。
