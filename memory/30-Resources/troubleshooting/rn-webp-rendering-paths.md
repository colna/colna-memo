---
title: RN 里 WebP 的可用渲染路径（expo-image 可以，原生 Image 不行）
date: 2026-09-18
tags: troubleshooting, react-native, ios, webp, assets, sitin-rn, luka
---

# RN 里 WebP 的可用渲染路径

给 RN 包做素材瘦身（PNG → WebP）时，**能不能转取决于这张图由哪个 Image 组件渲染**，
不是取决于文件类型。

| 渲染组件 | iOS 解 WebP | 说明 |
| --- | --- | --- |
| `expo-image` | ✅ | podspec 带 `SDWebImageWebPCoder`，iOS 有 `ios/Coders/WebPCoder.swift` |
| `@heyhru/rn-image` | ✅ | 内部就是 `expo-image` 的封装 |
| `react-native` 原生 `Image` / `Animated.Image` | ❌ | iOS 的 UIKit 不解 WebP（Android 原生支持） |
| `<SvgImage>`（react-native-svg） | ✅ | 底层走原生 webp 解码，但另一种用法，需单独确认 |

## 排查步骤（转之前先分类）

1. 列出真正随包的图（release 包里 `Payload/*.app/assets/...` 的清单，而不是仓库里
   所有素材 —— 仓库里可能有没被引用的死素材）。
2. 对每张图找 importer：`rg -l "assets/mascot/hamster-heart" src`。
3. 看 importer 的 Image 从哪来：`expo-image` / `react-native` / `@heyhru/rn-image`。
   注意 **per-prop 传递**（图作为 `mascotImage` 传给封装组件）要追到最终渲染组件。
4. RN 原生 Image 的那几张保持原格式，其余转 WebP。

## Metro / 类型

- `metro-config` 的 `assetExts` 默认包含 `webp`（可用
  `node -e '...getDefaultConfig(process.cwd()).resolver.assetExts.includes("webp")'` 复核）。
- 项目里有 `declare module "*.webp"`（见 `src/types/assets.d.ts`）才能 `import x from "./x.webp"`。

## 转码与验收

```bash
cwebp -q 85 -alpha_q 90 -metadata none in.png -o out.webp
```

- 带 alpha 的图**必须** `-alpha_q`，否则透明边缘会脏。
- 质量验收：`magick compare -metric PSNR in.png out.webp null:`（本批 26 张归一化
  误差 5e-5~1.5e-4，即 ~76-86 dB，肉眼无差别）+ 拼图目测（原图 | WebP 并排看渐变带）。
- 最终以 **iOS 模拟器逐屏截图**为准：深链打开 pet / quick-pick / chats / settings/logout
  等重素材屏，确认无红屏、无破图、透明正确。

## 本次踩的坑

- `rg` 默认跳过 `.gitignore` 里的 `node_modules`，查「某个依赖引了什么」要加
  `--no-ignore`；pnpm 的 `.pnpm` 是隐藏目录还要 `--hidden`。搜 `FontAwesome6` 一直
  0 结果就是因为这个（实际是自家 `chat-bubble.tsx` 直接 import，虚惊一场）。
- 统计 import 的正则别写 `[A-Za-z]+`：**模块名带数字**（`FontAwesome6`）会被漏掉，
  误以为多了 3 个字体是别人引入的。用 `[A-Za-z0-9]+` 或干脆搜 `vector-icons` 全串。
