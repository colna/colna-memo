---
title: expo-splash-screen 的 imageWidth 不是「图宽」：竖构图原生启动图会莫名小一半（imageWidth 是方形画布边长）
date: 2026-09-20
tags: [expo, react-native, ios, android, splash, 构建, troubleshooting, sitin-rn]
---

出处：sitin-rn `apps/luka`，2026-09-20（Expo SDK 57 / expo-splash-screen catalog 版，iPhone X 模拟器）。

# 现象

`app.config.ts` 里 `imageWidth: 244`，注释写着「与 JS 品牌屏的仓鼠 244 等宽」，但原生启动图上
整组构图只有约 **108pt** 宽 —— 比 JS 品牌屏（`src/app/splash.tsx`，仓鼠 244pt）小一半多，
冷启动换到 JS 屏时会「跳大」。量出来的像素：仓鼠 304px（3× 屏）≈ 101pt，字标 196px，标语 254px。

# 根因

**`imageWidth` 不是图片宽度，而是「把整张图装进去的正方形画布的边长」。**

- iOS：`expo-splash-screen/plugin/build/InterfaceBuilder.js`
  `const width = imageWidth; const height = imageWidth;` → storyboard 里 imageView 是
  `imageWidth × imageWidth` + `scaleAspectFit`；同时 `withIosSplashAssets.js` 把源图
  resize 成 `imageWidth * ratio` 的**方形** PNG（3× = `imageWidth*3` 见方）。
- Android：`withAndroidSplashImages.js` 把图 `contain` 进 `imageWidth * multiplier` 的方块，
  再合成到 **288dp** 画布中央。

所以竖构图的**可见宽度由高度决定**：

```
可见宽 = imageWidth × (图宽 ÷ 图高)
```

`244 × 732/1644 ≈ 108.6` —— 与实测吻合。给 244 只得到 109，怎么改都「不像 244」。

# 修法

按目标可见宽反推（sitin-rn 的 `splash.png` 是 732×1644，目标 244）：

```ts
[
  "expo-splash-screen",
  {
    imageWidth: 244 * 1644 / 732,        // = 548，iOS：548×548 方框里装 244×548 的构图
    android: { imageWidth: 244 },        // Android 画布 288dp 上限，548 会被裁
    image: "./src/assets/splash.png",
    resizeMode: "contain",
  },
]
```

- 两者能分开设：`getIosSplashConfig` / `getAndroidSplashConfig` 都把 `ios` / `android`
  子对象 merge 进 root（`{ ...rest, ...android }`），子对象的 `imageWidth` 覆盖 root。
- 548 这个值还有一层好处：3× 画布是 1644×1644，而源图 732×1644 **不放大**（1:1 落进去），
  仓鼠正好 244pt。
- Android 的可见宽被方形画布压到 `288 × 732/1644 ≈ 128dp` 上限；想更大只能另画一张
  接近方形的稿（原生层排不了活字，字是被烤进图的）。

# 验证

```bash
# 1) 生成物尺寸与 storyboard 约束
cd apps/luka && CI=1 npx expo prebuild -p ios --no-install
sips -g pixelWidth -g pixelHeight ios/Luka/Images.xcassets/SplashScreenLogo.imageset/image@3x.png
# 期望 1644×1644（旧：732×732）
grep -n "EXPO-SplashScreen" ios/Luka/SplashScreen.storyboard   # 期望 width=548 height=548

# 2) 真机/模拟器上量像素（×3 屏）
xcrun simctl io booted screenshot shot.png    # 仓鼠 304px → 683px（244pt 构图）
```

# 坑

`expo prebuild` 会 **clean 生成目录**（`Clearing ios / Creating native directory`）。luka 的
`apps/luka/ios` 上有本地手改的 `.devx` bundle id（绕 Personal Team 的 App ID 冲突，见
[[ios-appid-personal-team-conflict]]），跑完 prebuild 会被还原成 `com.lukasoc.luka.dev`，且
Pods 也没了 —— 验证这类「只影响生成物」的配置时，**先想清楚生成目录里有没有手改状态**，
或者干脆在临时副本里 prebuild。
