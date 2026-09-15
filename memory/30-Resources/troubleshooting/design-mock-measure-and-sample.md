---
title: 照着设计图做 UI：颜色怎么取、尺寸怎么量、什么时候别信图
date: 2026-09-10
tags: [troubleshooting, design, ui, workflow, sitin-rn]
---

# 照着设计图做 UI：取色、量尺与信不信图

出处：Luka 的 People / Profile / 发帖页三屏，全部是「用户丢一张图 → 实现」。

## 取色：用 ffmpeg 逐区域采样，不要肉眼猜

没有 Figma 源文件时（AI 出的稿、截图）也能把颜色取准：

```bash
ffmpeg -v error -i mock.png -vf "crop=<w>:<h>:<x>:<y>" -f rawvideo -pix_fmt rgb24 -
```

- **填充色**取区域内的**众数**（`Counter(...).most_common`）；
- **文字色**取区域内**最暗的像素** —— 描边≥2px 时它已经很接近真值，取平均会被背景拉淡。

实测收益：采出来的正文色 `#6B4A31` 和主题里的 `Colors.inkSoft`(#6B4D38) 几乎同一个
—— 说明这套稿本来就是照现有 token 画的，**能对上就别新造颜色**。

## 量尺：先确认这张图是不是真机比例

AI 生成的稿常是 1024×1536（比例 1.5），而 iPhone 是 393×852（比例 2.17）。这种图
**按像素换算出来的 pt 值没有意义** —— 按宽换算和按高换算能差 40%。

做法：
1. 只还原**层级与比例**（谁比谁大、占行宽的几成）；
2. 具体数值**锚定到仓库里已经做过的同类组件**（例如新列表行的字号锚到已迁移的
   `NearbyListRow`），而不是从图上量。

## 什么时候不照着做

稿子会有它自己的疏漏或含糊，照抄反而错：

- **同一屏把同一个事实说两遍**（名字下一行写了职业+城市，分割线下面又用图标列一遍）
  → 只留一处。
- **稿子没说点了去哪的入口**（一个带 `›` 的行）→ 要么接到一个真实存在的屏，要么做成
  就地生效的交互；**不要留一个点了没反应的东西**。
- **稿子上的词和 App 自己的词不一致**（稿子写 "Intro"，全 App 都叫 "Say hi"）→ 用
  App 自己的词，并在 PR 里说明。一个动作在一个 App 里有两个名字，比和稿子差一个词更糟。

以上每一条都要**在 PR 描述里写出来**，否则下一个人会以为是实现漏了。

## 素材

- 稿子里的小图标（心、徽标）优先**把 path 内联进组件**，而不是当资源图加载：一条路径
  而已，内联之后尺寸和颜色由调用方决定，也不必走资源管线和 @2x/@3x。
- 另一个 app 里已有的画法可以照着重画，但**不能 import，连注释里都不能提它的名字**
  （见 `sitin-rn` 的 boundaries 检查）。

## 把素材「贴到屏幕边上」：先算方盒的字母箱留白

稿子里那种「角色只露出一半、被屏边裁掉」的画法，实现上是负边距 + 外层裁切。第一次按
「内容区 padding 24 + 想裁掉 12 = 拉 36」去写，**截图里整只角色仍在屏内**，一点没被裁。

根因：共用组件的 `size` 常常**既当宽又当高**，里面再 `contentFit: contain`（RN 的
`expo-image` 同理）。素材宽高比 0.73 时，`size=160` 的方盒里图只有约 117pt 宽、左右各留
约 21pt **空白** —— 负边距要先吃掉这 21pt，才轮到「真的裁到图」。

```bash
magick identify -format '%wx%h\n' asset.png        # 原始尺寸
magick asset.png -trim -format '%wx%h %X%Y\n' info: # 透明边还剩多少
```

- 上面这条 `-trim` 是**先排除一种可能**：如果 trim 出来比原图小很多，那 21pt 里就混着
  透明像素（本项目的小崽素材入库前都裁过边，trim 出来是 +0+0，所以空白全来自方盒）。
- 目标落点用一句话定：`图左缘 = padding − 拉出量 + (size − 图宽)/2`。想裁 7pt、padding 24、
  图宽 117 → 拉 52。
- **改完必须截图核对**：这类偏差读代码看不出来，纸面上「拉了 36」和「被裁掉」听起来是一回事。
- **`size` 一改，落点必须跟着重算** —— 落点是从素材几何推出来的函数，不是常数。当前公式：
  `offset = 24(内容区 padding) + 0.189 × size − 2`（0.189 = 探头素材里那条身体竖边在 640 画布上的位置）。
  `size` 160 时落在 52，改成 214 就要变成 62。不重算的结果是「整只缩在屏内」或「半个身子被吃掉」。

## 「扁平 mockup」帧：只有比例可读，比就比同一屏里的同类量

有的 mockup 在 Figma 里就是**一张图**（`children: []`、唯一 fill 是 `IMAGE`，JSON 不到 1KB）
—— 量不到 fontSize / padding / 圆角，能读的只有**比例**。Luka 的 `78 · Edit name`（`932:1038`）就是这样。

1. 先把**屏幕那块矩形**找出来（这类稿常把 390×844 画在设备外框里，本例是 286×619 的奶油色区）。
2. 再挑**同一屏内可互相比较的两个量**去比，例如「角色画到的高度 ÷ 屏幕高」：
   稿 `152/619 = 24.6%`，实现 `471/2622 = 18.0%` → 小三分之一，`MASCOT_SIZE 160 → 214`，改后 24.0% ✓。
3. **两边量同一把尺**，换算系数（这稿是几倍图）直接被约掉 —— 不必先知道它是按哪个设备画的。

避开的坑：

- 别拿**跨元素**的两个量去比（角色高 vs 输入值字号）：两边的模糊边误差会相乘，读数不可信；
- 别把**另一屏**量出的比例搬到这屏（设备外框、内容密度都不同）；
- 稿子可能把角色**画在设备外框之上**（前爪压着黑边）—— 实机上屏边就是硬裁，
  「前爪完整可见」在实现里做不到，能保证的只有**身体那条竖边落在屏边**。

量 bbox 的稳法（比反复 `-trim` 强：`-trim` 会把相邻元素连着裁成一块，读不出「这条带子是什么」）：

```sh
# 阈值化成「非背景 / 背景」，再逐行投影；连续的非空行合成一个 run
magick shot.png -crop ${W}x${H}+${X}+${Y} +repage -fuzz 6% -fill white -opaque "$BG" \
  -fill black +opaque white -colorspace Gray -depth 8 pgm:- > /tmp/p.pgm
# 读这个 PGM：每行取「值 < 128」的像素，把连续行并成一条 run，打印 (absY 区间, absX 区间)
# 一条 run = 一条内容带（一行字 / 一只角色 / 一条下划线），误差 ±1px
```

`rowscan.sh <img> <x> <y> <w> <h> <bg> <fuzz%>` 就是上面这段的封装（`/tmp` 里，重启即丢，重写 20 行）。

## 没有截图时：用真字体量折行，别按字符数估

`figma-write-bridge` 的 spec 把 RN 屏幕折成绝对坐标，标题/副标题**折几行**直接决定它下面
所有内容的起点（Luka 的 scaffold：内容 = 副标底 + 30pt）。没有实机截图时，用 CoreText 拿
仓库里真字体的 advance 宽度量 —— **RN on iOS 就是 CoreText 排版，能逐字对上**：

- 字体在 `node_modules/@expo-google-fonts/nunito/<weight>/Nunito_<weight>.ttf`。注意
  `theme/fonts.ts` 的映射：`sansMedium` = **SemiBold 600**、`sansSemibold` = **Bold 700**、
  `headline` = **ExtraBold 800**（RN 按文件族名解析，写 `fontWeight` 不会自动换文件）。
- 约 20 行 Swift 就能量：`CTFontManagerCreateFontDescriptorsFromURL` 建字体 →
  `CTLineGetTypographicBounds` 量宽 → `CTTypesetterSuggestLineBreak` 在给定宽度下断行。
  不必装 fontkit / fontTools。
- **先校准再信**：拿一张实机截图里的已知折行验证脚本。Luka `07 · Name` 的副标题在 342pt
  可用宽下断在 `…show up in / Chat.`，脚本量 358.7pt、两行、断点一致 → 之后才用它量别的屏。
- 阈值附近（差 3~8pt）必须量：`This stays on your profile - pick what feels like you.`
  15px SemiBold 量 349.4（> 342，两行），`No judgement - it just helps us find your people.`
  量 329.0（一行）；30px ExtraBold 的 `What are you looking for?` 量 372.9 → 两行。字符数
  估算在这里必错，且错一行的代价是整屏内容下移 22~41pt。
- 折行结论落到 spec 里：**显式 `\n` + 两行的高度**（沿用 `25-edit-name.mjs` 的写法），
  别只给一行高度靠 Figma 自己折。

## `numberOfLines={N}` 的截断：词边界折行 + 末行换省略号，spec 里要写死

瀑布流卡片（Luka Feed / Nearby）的正文是 `numberOfLines={2}`。Figma 侧 bridge 插件给带
`width` 的 TEXT 节点设 `textAutoResize='HEIGHT'` —— 整段会自己往下长、压到作者行上。
截断必须在**写 spec 时**做掉，结论落成「显式 `\n` + 末行 `…`」。

怎么截得跟屏幕一样（两条都实测过）：

- **折行是词边界，末行才是字符级**：先把整段按可用宽（Feed 卡是 173pt）词级折行，取前
  N 行；最后一行末尾换成 `…`（丢掉行尾空格）。下一行开头的词**不会被拉上来**填满，
  所以结果是 `actually like out of the…`，不是 `actually like out of the kiln.…`。
- **用 TextKit 复现**（AppKit，20 行）：`NSTextContainer(maximumNumberOfLines=N)` +
  `NSLayoutManager`，段落样式用 **`byWordWrapping`**，再枚举 line fragments 取每行文本。
  别用 `byTruncatingTail` 建段落样式 —— 那会让 framesetter 把整段压成**单行**再截，
  量出来的折行全错（`CTFramesetterSuggestFrameSizeWithConstraints` 也一起错）。
- 校准过：Luka 15px Bold / 173pt，`Finally pulled a glaze I actually like out of the kiln.`
  → 两行 `Finally pulled a glaze I` / `actually like out of the…`。

## `flex-wrap` 的临界值必须按真字宽量

Nearby 卡的兴趣 chip 是 `flex-row flex-wrap gap-1.5`，可用宽 = 169 − 24 = 145。Naomi 的
`Cooking` + `Ceramics` 真字宽 45.3 / 50.6，加两侧 11pt 内边距 = 67.3 + 73.6，**再加 6 的
gap 是 146.9 > 145 → 换到第二行**（差 1.9pt 也是换行）。按字符数估会判成一行，卡里就少
一行 chip、名字上移 32pt。量法同上一节（`CTLineGetTypographicBounds` + padding）。
