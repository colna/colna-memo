---
title: sitin-next PWA — Figma 图片一律转 webp,不用 SVG
date: 2026-07-02
tags: [sitin-next, app-pwa, figma, webp, 约定]
---

# 规则:app-pwa 从 Figma 取图,先下载再转 webp,不直接用 SVG

## 约定(硬规则)

在 **sitin-next 的 app-pwa** 项目里,凡是从 Figma 拿的**图片 / 图标**,都要:

1. 从 Figma 把图片/图标**下载下来**;
2. **转成 webp** 再引入使用(`import xxx from "@/assets/images/.../icon_xxx.webp"` + `<img>`);
3. **不要**把 SVG(内联 `<svg>` 或 `.svg` 文件)直接用在组件里。

图标放约定目录,例:聊天相关放 `packages/app-pwa/src/assets/images/chat/`,命名 `icon_<模块>_<名字>.webp`(如 `icon_phototask_camera.webp`)。

> 为什么:与 app-pwa 既有资源风格一致(现存图标都是 webp);webp 体积小(这批图标 lossless 也 <1.6KB);避免内联 SVG 撑大组件 + 一堆 path。

## 落地流程(本机验证可行)

本机(macOS)**没有** cwebp / ImageMagick / rsvg / brew / Python-PIL;**`sips` 假装支持 webp 输出但实际不写文件**(命令返回 0、无产物,别信)。可行链路:

1. **下载**:figma MCP `download_figma_images`,按 nodeId 下 **PNG**(`pngScale: 4` 拿高清),存到临时目录。
   - 图标是 vector(IMAGE-SVG)节点也能直接渲成 PNG。
2. **转 webp**:在 scratchpad 临时 `npm i sharp`(预编译带 libvips,自带 webp 编码),node 脚本:
   ```js
   const sharp = require("<scratchpad>/node_modules/sharp");
   await sharp(`${src}/x.png`).webp({ quality: 92, lossless: true }).toFile(`${dst}/icon_x.webp`);
   ```
3. 产物落到 app 的 assets 目录;**`sharp` 只装在 scratchpad,不进 app 依赖**;临时 PNG 用完即删。

## 组件用法

轻量 `Icon` 组件(定宽高的 webp `<img>`),需要旋转就加 `animate-spin`:
```tsx
const Icon = memo<{ src: string; size: number; className?: string }>(({ src, size, className }) => (
  <img src={src} alt="" style={{ width: size, height: size }} className={className} />
));
```

首次落地见 `PhotoTaskDrawer.tsx`(commit `21fc07bc`,PR #517)。相关:[[pwa-mobile-gesture-media]]。
