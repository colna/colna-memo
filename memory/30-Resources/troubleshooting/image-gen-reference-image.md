---
title: image-gen 参考图怎么选、prompt 怎么配套
date: 2026-09-11
tags: troubleshooting, image-generation, image-gen, prompt, luka, mascot
---

# image-gen 参考图（`--ref`）的坑

出角色一致的 UI 设计图时，`--ref` 传什么、prompt 怎么跟着改。踩过的都在这。

## 1. 不要指 MetaBot 的下载目录 —— 每轮会被清空

用户在飞书发的图落在 `metabot-downloads-max/`，**下一轮对话它就没了**：

```
参考图不存在:/var/folders/.../metabot-downloads-max/img_v3_0215d_*.png
```

`metabot-outputs-max/<chat_id>/` 同理，上一轮生成的图也不在了。

**修法不是让用户重发，是换成仓库里常驻的素材。** Luka 的例子：

```
sitin-rn/apps/luka/src/assets/mascot/
  hamster-{waving,heart,curious,pointing,standing,camera,coach,pin,aside}.png
  hamster-bowl-hero.png  deck-pet-bubble.png
```

这些比用户给的角色卡**还准** —— 角色卡是提案，这些是产品里真在用的那只。
**以后 Luka 出图的 `--ref` 一律指这个目录。**

## 2. 去背素材当 ref，要在 prompt 里明确「忽略它的背景」

角色卡是完整海报（要防「抄版式」），去背 PNG 是另一种翻车方式：**黑色/透明 matte
会被当成画面内容带进结果**。master 里那段参考图说明要改写：

```
ONE ATTACHED IMAGE is a CUT-OUT MASCOT ASSET, supplied as CHARACTER REFERENCE
ONLY. Take the hamster's look from it — his proportions, fur pattern, eyes and
pink heart nose — and NOTHING else. Its background is a flat cut-out matte:
IGNORE that background completely, never carry any dark or black field into the
result. The output is a phone UI screen on a cream ground.
```

negative 补上：`black background, dark matte field, cut-out background, checkerboard`。

> 对照：参考图是**角色卡/海报**时，防的是抄版式 ——
> 「never copy its layout, its panels, its swatch rows, its Chinese labels, its
> cream poster background or its typography」。两种 ref 的防御条目不一样，别混用。

## 3. ⚠️ 选完 ref 要回头核一遍 negative 有没有和它打架

这条最容易漏。Luka 的 negative 里常年有一条：

```
standing upright on two legs
```

（防模型把圆球仓鼠画成拟人站姿。）而 `hamster-waving.png` **本身就是站姿挥手**——
ref 和 negative 直接对冲，模型两头不讨好。

**修法是按 ref 拆 negative**，不是改 ref：

| 版本 | ref | negative |
|---|---|---|
| 挥手版 | `hamster-waving.png`（站姿） | 单独去掉 `standing upright on two legs` |
| 抱心版 | `hamster-heart.png`（坐姿圆球） | 保留完整 negative |

所以 negative 不是一个全局文件，是**每版一份**（`negative-feed-a.txt` /
`negative-feed-b.txt`）。

## 4. prompt 拆三段拼接，别写成一个大文件

```
master.txt      # STYLE BIBLE，跨屏逐字不变 —— 一致性全靠它
screen-x.txt    # 只写这屏独有的
negative-x.txt  # 按 ref 和本屏特有翻车方式定制
cat master.txt screen-a.txt negative-a.txt > prompt-a.txt
```

改一屏只动中间那段，master 绝不在版本之间改。

## 5. 用 python 补丁改 prompt 时，assert 要贴着真实换行

用 `sed` / python `str.replace` 改这些文件时，长句在文件里是**带换行的**：

```python
# ✗ 匹配不到
s.replace("standing upright on two legs, ", "")
# ✓ 文件里实际是 "thin limbs, standing\nupright on two legs, realistic rodent"
s.replace("thin limbs, standing\nupright on two legs, realistic rodent", "thin limbs, realistic rodent")
```

**每个 replace 后面跟一个 `assert`**，否则 `cat` 拼出来的 prompt 会静默少一整段
（实测：negative 段整个丢了，文件小了 1.4KB 才发现）。

## 相关

- [image-gen 透明底与抠图](image-gen-transparent-cutout.md)
- [AIGC-Studio 生图接口排错](aigc-studio-image-gen.md)
- [交付物先确认消费方](deliverable-consumer-first.md) —— 出图 prompt 只写视觉，
  不写 Interaction / Data 两段（那是可交互原型工具才要的）
