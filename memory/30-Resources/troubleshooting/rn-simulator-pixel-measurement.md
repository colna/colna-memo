# React Native：用模拟器截屏量 UI 几何（pt ↔ px）

**场景**：要判断「A 有没有压到 B 的边」「某个绝对定位的偏移量该取多少」，而模拟器点不动
（`osascript` 没开辅助功能权限）、1x 预览里肉眼也看不出来。

**结论**：截屏 + `magick` 逐列 trim 能到像素级，但**必须先做一次标定实验**，否则拟合出来的
「圆弧」很可能是错的。

## 步骤

1. 截屏：`xcrun simctl io booted screenshot /tmp/x.png`。模拟器是 3x —— iPhone 17 出图
   1206×2622，即 402×874pt、3px/pt。
2. **标定 pt→px、同时验证属性生效**：把待查参数临时改成夸张值（例：`PET_DROP = 30`），等
   Fast Refresh 后再截一张，量同一列的位移。位移 ÷ Δ参数 = pt→px（本次量到正好 3）。**这一步
   顺便回答「这个属性到底有没有生效」** —— 只靠读代码很容易以为它生效了。
3. 量边缘：

   ```sh
   magick x.png -crop 2x$H+$X+$Y +repage -fuzz 1% -trim -format '%Y %h' info:   # 底 = Y0 + Y + h
   ```

   `-fuzz` 用 1%，大了会把抗锯齿边一起吃掉（6% 能吃掉 10px 以上）。

## 坑

- **列底会被下面的文字污染**：trim 给的是 bbox 不是连续段。窗口要卡在文字上方 —— 名字、按钮
  文字都在底下等着，一次没卡好就会把「文字顶边」当成「这个元素的下沿」。
- **低对比的圆边量不准**：照片内容一旦接近底色（浅花瓣、天空、浅灰），trim 停的位置就开始飘，
  同一张图两列拟合出的圆心能差十几 px。**优先量对比强的边**（角色的毛 vs 奶油底），再用它反推
  容器几何（本次就是先量到小崽平切底边、再反推出圆盘底切点）。
- **视频素材会动**：单帧不代表整段，连拍几张看范围；判断「有没有越界」要用**并集包围盒**而不是
  某一帧。
- **离线合成的前提是模型对**：把素材叠到目标几何上做预览很有用，但模型错了照样「看着像那么
  回事」。先用真实截屏核对一次再信它（见 [rn-offline-composite-for-overlap.md](rn-offline-composite-for-overlap.md)）。

- **截图右上/右下那颗齿轮不是 App 的 UI**：是 `expo-dev-client` 的 Dev Menu 悬浮按钮（FAB），iOS 上**默认开启**（`node_modules/expo-dev-menu/ios/Modules/DevMenuPreferences.swift` 里 `fabDefault ?? true`）。它会挡住要量的区域，关法三种：
  ① Dev Menu → **Tools button** 开关（UI 里点掉，持久化在 UserDefaults）；
  ② 只针对本机模拟器：`xcrun simctl spawn booted defaults write <bundleId> EXDevMenuShowFloatingActionButton -bool false`，重启 App 生效（恢复：`defaults delete` 同一个键）；
  ③ 项目级对所有人生效：`app.config.ts` 的 `ios.infoPlist.EXDevMenuShowFloatingActionButton: false`（要 prebuild / 改 `ios/<App>/Info.plist`）。
  它还**可以拖动**，所以同一 App 两张截图里位置可能不同 —— 别当成布局变化。

- **盖住整屏的是 Dev Menu 的引导弹窗**（首次装 dev 包 / 每次未确认前启动都会有），比 FAB 更挡事：它自己带一个 Continue 按钮，而模拟器上的点击注入常常打不进去（见 [[ios-simulator-keyboard-and-taps]]）。**绕法是不点它，直接写偏好**：
  `xcrun simctl spawn <UDID> defaults write <bundleId> EXDevMenuIsOnboardingFinished -bool YES` → `simctl terminate/launch`，弹窗不再出现（键名出自 `node_modules/expo-dev-menu/ios/Modules/DevMenuPreferences.swift`）。

## 到不了的那一屏 / 那一个分段怎么绕

- 深链：`xcrun simctl openurl booted "<scheme>://<path>"`（例：`luka://me` 直达 Me tab）。
- 分段/子 tab 存在 `useState` 时深链进不去：**临时把默认值改成目标分段 + `simctl terminate`
  → `simctl launch` 冷启动**（Fast Refresh 会保留 state，改初始值不生效），截完把这一行撤掉。
## 补充（2026-09-22，给每个块上色 + 逐行数色：一次拿到整屏的分块高度）

给待查的每一块临时套一个纯色背景（`style={{ backgroundColor: "#FFFF00" }}` 这类，块内文字仍可读），
截屏后把图缩到「1px = 1pt」再逐行数颜色，就能一次拿到所有块的 top/bottom：

```sh
# 缩到 393 宽（原图 1179 → 3px/pt 变 1px/pt），找每个目标色的连续行段
magick probe.png -resize 393x -depth 8 txt:- | ...   # 用 python 解析 "x,y: (r,g,b)" 逐行归类
```

比逐列 `-trim` 强的地方：**不怕低对比边、不怕文字污染，也不怕绝对定位的浮层**（只要先给容器上色）。
本次用它量出「权益卡只剩 9pt、可用空间全被固定块吃掉」，比读代码猜快得多。

**先 `magick identify x.png` 拿真实尺寸**：`-crop` 的窗口超出图高时只报警告、不报错，会让人
把 375×812 的机器当成 393×852 算一晚上（本次的坑，浪费了一轮）。设备名（`iPhone X (11 Pro eq)`）
也不可信 —— 以 `identify` 为准，3x 时 `px / 3 = pt`。

## 补充（2026-09-22，dev build 里 `console.log` 到不了 `log stream`）

想用 `onLayout` + `console.log` 读数值时注意：**RN 0.86 的 dev build 把 console 走 Metro 的
调试器通道，不落 `xcrun simctl spawn booted log stream`**（`eventMessage CONTAINS "[tag]"` 里一条
都收不到）。要读数值就**渲染到屏幕上**（临时的 `Text` 叠加层 / `Alert`），截屏后用眼睛或 OCR 读。
`onLayout` → `setState` 的读数条本身还会在布局变动时抛 `Cannot read property 'layout' of null`
的红屏（量完即撤，别留在分支里）。最省事的仍然是上色法 —— 它不需要 App 输出任何东西。

## 补充（2026-09-22，被 `Stack.Protected` 挡住的屏：临时放宽守卫）

`luka` 的路由（如 `/paywall/likes`、`/paywall/visitor`）在 `Stack.Protected guard={authed &&
profileComplete && !pendingDeletion}` 里，账号没补完资料时深链也进不去（这正是设计）。只想截屏
核对布局时，**临时把那条 `guard` 改成 `true`**，冷启动后深链进入，截完立刻 `cp` 备份还原 ——
但**先确认这份 worktree 没有别的会话在写**（本次另一个会话正在改同目录的 `paywall/index.tsx`），
临时改动留在共享工作区里可能被别人的提交带上去。

