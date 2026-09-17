---
title: react-native-screens formSheet：内容必须是单个 ScrollView（iOS 26）
date: 2026-09-17
tags: [troubleshooting, react-native, expo-router, ios26, sitin-rn]
---

# expo-router 的 formSheet：内容结构不是自由布局，iOS 26 会强制重排

**坑**（2026-09-17，luka 礼物抽屉从自绘 Modal 迁到原生 formSheet）：把 sheet 内容做成
「表头 TabBar + 网格 ScrollView + 页脚」三段式，模拟器上 **TabBar 被压成 0 高、文字溢出
到网格上面叠在一起**，Metro 里还有一条警告：

```
[RNScreens] FormSheet with ScrollView expects at most 2 subviews.
Got 8 for container: RNSSafeAreaViewComponentView. This might result in incorrect layout.
```

**根因**（`node_modules/react-native-screens/ios/RNSScreenContentWrapper.mm`，iOS 26 分支）：
formSheet 会自己找内容里的 `RCTScrollViewComponentView`，并按它在外层容器里的下标强行设
frame：

- 下标 0（第一个子节点）→ 整块尺寸；
- 下标 1 → 把第一个子节点当**表头**，滚动区高度 = 整块 − 表头高；
- 其余 → 不处理（Fabric 的 flex 结果和原生强设的 frame 打架，于是重叠）。

**修法**：整张 sheet 只放**一个** ScrollView，其它东西全做它的子节点 —— 表头用
`stickyHeaderIndices={[0]}` 钉顶（记得给表头实底背景，否则内容从字底下透出），页脚用
`contentContainerStyle={{ flexGrow: 1 }}` + 一个 `flex-1` 弹性 View 沉到底部，容器样式
（底色等）放在 ScrollView 的 `style` 上，否则矮内容下方会露出系统默认灰。

**判据**：写 formSheet 路由时先问「这段内容在 Fabric 树里是不是**恰好一个 ScrollView**」。
奖励是原生拖拽 / 遮罩 / 键盘都由系统接管，比自绘 Modal 省掉几十行手势与动画代码。

**验证手法**：给 worktree 单起一个 Metro（`--port 8082`），用 dev-client 深链
`luka://expo-development-client/?url=http%3A%2F%2F127.0.0.1%3A8082` 指过去，再用业务深链
（如 `luka://gift?peerId=…`）直开目标路由，`xcrun simctl io <device> screenshot` 截图核对，
不必手点。注意挑对方没在用的模拟器，验完把设备切回对方 Metro。
