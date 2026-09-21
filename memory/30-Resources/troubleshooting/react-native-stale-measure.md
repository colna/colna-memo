---
title: React Native 测量缓存停在旧位置（祖先位移不触发子 onLayout）
date: 2026-09-21
tags: [react-native, layout, measure, safe-area, troubleshooting]
---

# React Native 测量缓存停在旧位置（祖先位移不触发子 onLayout）

## 症状

浮层挖洞/气泡、tooltip、锚定 UI 用 `measureInWindow` 缓存坐标时，**整层偏移了一个状态栏**
（约 44pt），目标元素本身渲染正常。

2026-09-21 sitin-rn `apps/luka` 新手引导：Chats 第 4 步（页头爪印）气泡+高亮整体比爪印高 44pt。

## 根因

`onLayout` 只在**元素相对父容器的** frame 变化时触发。以下位移不会触发目标的 `onLayout`：

- `SafeAreaView` 的原生 padding 比目标的首次布局**晚一拍**生效 → 页头整体下移 44pt，目标相对父容器没变；
- 任何由祖先 padding / 定位变出来的位移。

于是「只在 `onLayout` 里测量」的目标，坐标会永久停在 padding 生效前的那一拍（页头在 window y=0）。

## 修法

`onLayout` 之后**再补量一拍**（sitin-rn 用 300ms，每次 onLayout 重置定时器、卸载清理）：

```ts
const guideTargetOnLayout = useCallback(() => {
  measure();
  requestAnimationFrame(measure);              // 首屏布局还没提交到窗口
  if (settleTimer.current) clearTimeout(settleTimer.current);
  settleTimer.current = setTimeout(measure, 300); // 祖先驱动的位移
}, [measure]);
```

`measureInWindow` 本身是**实时**的 —— 再调一次就拿到当前位置，所以补量是充分修法。
若消费方是「等坐标稳定再快照」的样式（sitin-rn 的 `PageGuideHost`），补量会让稳定窗口
自动顺延，不需要另加时序。

## 怎么在没有设备时定位（截图像素取证）

1. 由气泡宽度反推比例尺：sitin-rn 里气泡 `bubbleWidth` 固定 320pt，白块实测 598px →
   ≈1.87 px/pt；再用 dp 已知的内边距（`GUIDE_BUBBLE_MARGIN=16`）交叉验证左右夹边。
2. 反推目标应在的位置：`placement=below` 时 `top = rect.bottom + 14`，倒过来算 rect.y；
   若算出来正好是「没算安全区」的位置（如 (54−40)/2=7），基本就锁定是这类 stale measure。
3. 找一条**已知真实几何**的参照物验证「页面渲染没问题」：sitin-rn 里用团队头像的未读徽标
   （`-top-1 -right-1`，可算出精确 bbox）与截图实测对齐，证明只有缓存的坐标错。
4. 若截图上有半透明浮条/系统 UI 压着目标区域，洞会从它下面透出亮块 —— 亮块边界能反过来
   验证洞的位置（本例透出亮块右沿 702px = 洞右沿 363pt）。

## 相关

- 修法落地：`apps/luka/src/hooks/use-guide-target-measure.ts`、`apps/luka/docs/page-guide.md`
- 同类坑（快照竞态）：`50-Daily/2026-09-20.md` 12:03 条目
