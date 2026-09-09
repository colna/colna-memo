---
title: 带 alpha 的 MOV 压缩 —— pix_fmt 会撒谎,只能解一帧实测
date: 2026-09-09
tags: [troubleshooting, ffmpeg, video, alpha, sitin-rn, luka]
---

# 带 alpha 的 MOV:判定与压缩

## 坑一:`ffprobe` 的 `pix_fmt` 对 HEVC-with-alpha 一律报 `yuv420p`

Apple 的 HEVC-with-alpha 把 alpha 放在**辅助层**,不体现在流级 `pix_fmt` 上。

```
$ ffprobe -show_entries stream=pix_fmt apps/luka/src/assets/pet/pet.mov
pix_fmt=yuv420p        # 骗人的,这个文件是有透明的
```

反过来某些 `rgba` 源每个像素都不透明。**两个方向都会错,不要按 `pix_fmt` 判 alpha。**

**正确做法:真解一帧数透明像素。**

```bash
ffmpeg -v error -i in.mov -frames:v 1 -vf "format=rgba,scale=128:128" \
  -f rawvideo -pix_fmt rgba - | python3 -c '
import sys; d=sys.stdin.buffer.read(); a=d[3::4]
print(sum(1 for v in a if v<8)/len(a))'
```

`pet.mov` 实测透明占比 0.5537,`panda.mov` 0.6108。

## 坑二:「ffmpeg 软解会丢 alpha 辅助层」已经过时

`apps/luka/src/assets/pet/README.md` 与 `scripts/render-pet-animation.mjs` 都写着这句,
并为此专门配了 `scripts/verify-pet-alpha.swift` 走 AVFoundation 校验。

**在 ffmpeg 9.0.1 上已经不成立** —— 实测 `pet.mov` 左上角解出 `RGBA = 0,0,0,0`,中心
`243,183,31,255`,alpha 完整。所以现在可以直接用 ffmpeg 重压带 alpha 的 MOV。

老版本 ffmpeg 仍会静默拍平成黑底、**中途不报错**,这是最危险的一种失败。稳妥写法是
「ffmpeg 解不出 alpha + AVFoundation 解得出」时**拒绝执行**并提示升级 ffmpeg。

**2026-09-09 在第二台机器上坐实了版本差异。** 另一台 Mac(`/Users/colna/WORK/sitin-rn`)
上对**线上在用、确定带透明**的 `pet.mov` 跑同一条命令,读到 `0.0`:

```bash
ffmpeg -v error -i apps/luka/src/assets/pet/pet.mov -frames:v 1 \
  -vf "format=rgba,scale=64:64" -f rawvideo -pix_fmt rgba - | \
  python3 -c "import sys;d=sys.stdin.buffer.read();a=d[3::4];print(sum(1 for v in a if v<8)/len(a))"
# 0.0        ← ffmpeg 9.0.1 上是 0.5537
```

**判别手法(便宜且可靠)**:拿一个已知带透明的文件当对照组跑一遍。对照组读到 0
= 解码器不行;对照组正常而目标读到 0 = 文件真丢了 alpha。

**由此引出的设计教训**:压缩脚本的输出校验**不能只用 ffmpeg**。同一段素材,输入是
ProRes(谁都解得动)读到 0.7980、输出是 HEVC 读到 0 —— 这个不对称本身就是信号。
`compress-mov.sh` 已改成 ffmpeg 读不到时用 AVFoundation 复核(commit `4f1441a4`)。

## 坑三:`ffprobe` 不按 `-show_entries` 的顺序输出字段

```bash
# 错:CODEC 会拿到 width 的值
read -r W H PIXFMT CODEC < <(ffprobe ... -show_entries stream=width,height,pix_fmt,codec_name -of default=nw=1:nk=1 ...)
```

ffprobe 按它自己的固定顺序输出(`codec_name` 在 `width` 前面)。**必须带 key 输出、按 key 取值**
(`-of default=nw=1` 然后 `sed -n 's/^width=//p'`),不能按位置读。

## 坑四:`hevc_videotoolbox` 的三个参数不能自由发挥(ffmpeg 7.1.1 会编出打不开的文件)

2026-09-09 在另一台 Mac(ffmpeg **7.1.1**)上,`compress-mov.sh` 编出的 MOV 让
AVFoundation 报 **`Unable to start decoder`** —— 文件根本打不开;同一份 ProRes 4444
源在本机 ffmpeg 9.0.1 上完全正常。对照组 `pet.mov` 用 swift 校验读到 `0.5612`,
证明 swift 环境没问题,是产物坏了。

差别在三个参数。仓库里两份能出好文件的脚本(`optimize-pet-mov.mjs`、
`render-pet-animation.mjs`)**都不这么写**:

| | 错误写法 | 正确写法 |
|-|-|-|
| 软编回退 | `-allow_sw 1` | **不给** —— VideoToolbox 的软件 HEVC 编码器不支持 alpha,回退后静默产出坏文件 |
| 码率 | `-q:v <0-100>` 恒定质量 | `-b:v 400k`(320×320 的档,按面积等比放大) |
| 像素格式 | `-pix_fmt bgra` | 滤镜链里 `format=gbrap,premultiply=inplace=1,format=bgra` |

另外补 `-map 0:v:0 -map_metadata -1 -movflags +faststart`。**premultiply 必须和裁剪、
缩放拼在同一条滤镜链里** —— 颜色和 alpha 走同一次重采样,边缘才不会错开一个像素出白边。

**教训**:这类硬件编码器的参数不要自己推,直接抄仓库里已经跑通的那一份。
本机能跑 ≠ 参数正确,只说明本机的 ffmpeg 兜住了。

## 编码参数

| 场景 | 参数 |
|-|-|
| 保 alpha(仅 macOS) | `-c:v hevc_videotoolbox -allow_sw 1 -q:v <40-95> -alpha_quality 0.9 -pix_fmt bgra -tag:v hvc1` |
| 普通 H.264 | `-c:v libx264 -crf 23 -preset slow -pix_fmt yuv420p -movflags +faststart` |
| 更小的 H.265 | `-c:v libx265 -crf 23 -preset slow -tag:v hvc1` |

`hevc_videotoolbox` **不吃 CRF**,走 `-q:v 0-100`(越大越清晰)。

## 工具

通用脚本:`/Users/max/Dev2/zhangzheng/tools/compress-mov.sh`
—— 自动实测 alpha、可选按全帧 alpha 并集包围盒裁紧(`--crop-alpha --square`)、
输出后回测透明占比,丢了 alpha 直接 exit 1。

仓库里专用的那份是 `sitin-rn/scripts/optimize-pet-mov.mjs`,只吃带 alpha 的母版、
写死缩到 320×320,不带 alpha 的源直接抛错。
