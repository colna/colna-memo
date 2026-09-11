---
title: RN/Yoga：自适应高度容器里的 height:100% 会把兄弟节点顶出屏幕
date: 2026-09-11
tags: [troubleshooting, react-native, yoga, layout, uniwind, sitin-rn]
---

# 自适应高度容器里的 `h-full`

出处：sitin-rn `apps/luka`，Profile（Me tab）名字行右侧加「Edit profile」入口后，
头像下面的整块内容消失（MeTabs / Daily bonus / 菜单卡全不见），**无红屏、无 JS 报错**。

## 症状

- 「下面那一块没了」，但**不是没渲染**：那块区域仍然占着高度。量截图看：头像底 ≈359pt、
  名字行 ≈432pt，中间空 ~73pt —— 正好是 `MeTabs` 的 `mt-5`(20) + 46 高。位置对得上，
  只是没有像素。
- 名字被**垂直居中**到行中央（上下留白大致相等）—— 这是「这一行被撑高了」的指纹。

## 根因

```jsx
<View className="w-full flex-row items-center">            {/* 高度自适应 */}
  <Pressable className="h-full items-end justify-center">  {/* height: 100% */}
```

Yoga 对「父容器高度自适应 + 子元素百分比高度」没有确定解，会按**可用空间**解算：这一行于是
吃掉整块可用高度，把它后面的兄弟节点全推到折叠线以下。RN 不会为此打任何警告。

## 修法

- 要「撑满这一行」用 **`self-stretch`**（`align-self: stretch`）—— 相对行的**已解析高度**，
  不引入循环 ✓。
- 只要点击热区够大，直接**不写高度**（内容高 + `hitSlop`）也可以。
- 判据：**父容器高度自适应时，永远不要给子元素写 `h-full` / `h-screen` / `height: "100%"`。**

## 怎么一眼确认（截图取证）

```sh
xcrun simctl io booted screenshot /tmp/s.png
# 可疑区域：stddev≈0 且 min 偏高 = 真空白；有内容就有明显 stddev 与更暗的 min
magick /tmp/s.png -crop 100%x30%+0+1500 -format '%[fx:standard_deviation] min=%[fx:minima]' info:
```

修前 `stddev=0.024 min=0.647`（那片只剩右下角一颗 dev 齿轮），修后 `stddev=0.084 min=0.090`。

## 连带教训

**改屏内布局时，只检查自己改的那一行不算验过。** 这次只看了名字居中与按钮右缘距离，
漏看整屏，回归是被用户先发现的。整屏截图 + 上面那条像素判据，成本几乎为零。
（另见 [design-mock-measure-and-sample.md](design-mock-measure-and-sample.md)：量稿/量截图那套工具。）
