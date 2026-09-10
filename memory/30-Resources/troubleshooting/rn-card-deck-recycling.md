---
title: RN 卡片堆叠：换卡闪动、状态泄漏与被丢掉的守卫
date: 2026-09-10
tags: [troubleshooting, react-native, reanimated, sitin-rn]
---

# RN 卡片堆叠：换卡闪动、状态泄漏与被丢掉的守卫

出处：sitin-rn `apps/luka` 的划卡容器 `CardStack`（PR #465）。写堆叠式划卡（一屏一张、
背面垫下一张）时全都会碰上，和具体 App 无关。

## 一、划完之后闪一下 —— 两个独立成因，会叠在一起

### 1. `key` 是「人」而不是「槽」

```tsx
<Animated.View key={candidates[index].id}>   // ⛔
```

`index` 一加一，key 就变 → React **卸载重建整棵子树**。讽刺的是下一个人的卡本来已经
垫在底下、图也解好码了，但它在树里是另一个位置的另一个元素，新挂上来的当前卡拿不到，
只能重走一遍 `Image` —— 那一两帧画的是 `loadingColor` 占位色。

**修法：按槽渲染。** 永远只有 0、1 两个槽，`index` 每加一两个槽交换正反面
（`front = index % 2`），槽的 key 固定为 `slot-0` / `slot-1`。背面槽转正时演的还是同一个
人、同一棵子树，照片一张都不用重新解码；换成新人的是**转到背面**的那个槽，被正面完全
盖住，重新加载看不见。

叠放次序用 `zIndex` 决定，**不要靠调换 JSX 顺序** —— 顺序固定，正反面交替时 React 只改
几个 prop，连原生视图的挪位都省了。

### 2. shared value 的写入比 React commit 早一帧

```js
setIndex(next);          // React 状态：要等 render → commit
translateY.value = 0;    // shared value：下一个 UI 帧就生效
```

reanimated 的写入不依赖 JS 的 render 工作，路径短；`setIndex` 要走完 render + commit。
中间那一帧的状态是 **index 还是旧的、transform 已经归零** —— 刚飞出去的卡以 opacity 1
回到原位画了一帧。

**修法：让归零发生在「看不见的那张」上。** 两个槽各带自己的位移，归零放进
`useEffect([index])` 归**刚转到背面**那个槽的零 —— effect 在 commit 之后跑，此时它确实
已经在背面，改它不可见。

> 只修这一条治不了根：那样只是把「旧卡回闪一帧」换成「新卡在屏外一帧」，照片重解码
> 那块完全没动。修法一才是主因，且修完之后这条竞态自然消失。

**前提要自己确认**：「背面被盖住所以看不见」成立的前提是两张卡尺寸严格相同。本例里
`discoveryCardLayout` 的 `cardHeight` 只由 `maxHeight` / `cardWidth` 决定，与内容无关
（内容多少只改照片高度），且卡片用的是写死的 `height: layout.cardHeight`。**如果卡高随
内容变，背面会从底部露出来，上面这套说法就不成立。**

## 二、槽复用会把子组件的 state 留给下一个人

这是修法一的**直接代价**，必须同时处理，否则换来一个更糟的 bug。

本例：`CandidateCard` 里 `greeting` 一旦置 true，代码里**没有任何地方置回 false**。槽复用
之后表现为「对 A 打过招呼后，轮到这个槽的每个人 Say hi 都点不动、卡片也点不开」
（`disabled={greeting}`），`favourite` 同理会把没收藏过的人显示成已收藏。

**修法不是加重置 effect，是给内层组件按人加 `key`**：

```tsx
<Animated.View key={`slot-${slot}`}>        {/* 槽稳定：手势、位移、原生视图不动 */}
  <CandidateCard key={candidate.id} ... />  {/* 人变了才重建：只发生在背面那张 */}
</Animated.View>
```

槽只在转到背面时才换人，所以这个 key 只让被完全盖住的那张重建，正面那张照旧复用。
**一行 key 同时拿到「不闪」和「状态干净」。**

## 三、换掉一个老容器时，先把它的守卫列出来

`CardStack` 重写时把 `CardScroll` 攒下的一批守卫一起丢了，`/code-review` 一次抓出 10 条。
这些都不是新容器「设计得不好」，而是**老容器每一条都对应一次线上问题**，重写时看不见：

| 守卫 | 丢了之后 |
|---|---|
| 挂载时 announce 首卡 | 第一个人的曝光永不上报、下一张不预取、整批预加载点不着 |
| 游标夹进 `[0, count-1]` | 恢复位置越界 → 一屏空白且 `onRunOutOfCards` 不触发，没有出路 |
| `animating` 共享值 | 飞出途中第二次触摸打断 `withTiming` → `done === false` → **已过完门控的提交凭空消失** |
| 手势 `useMemo` | 后台补数据触发父级重渲染 → 手指按着时丢掉触摸 |
| `remove()` 里的 announce | 换人后曝光时长算成上一张的 |
| 阈值常量从容器导出 | 见下 |

**可复用的动作**：重写容器前，把老容器里所有 `useMemo(..., [])`、共享的布尔闸、以及
带 `biome-ignore` / 长注释的地方**逐条抄成清单**——那些注释就是事故记录。

### 阈值常量必须从「定义它的那一侧」导出

`useSwipeThresholdHaptic(commitAt)` 的注释里记着一次事故：它曾写死 `0.5` 而容器在 `0.05`
commit，那颗震动**从来没响过**。换容器时沿用旧的 `SCROLL_THRESHOLD = 0.05`，而新容器报的
进度已按 22% 归一化（划走线就是 `1`）——方向反过来，变成**手一动就震**。

→ 由容器 `export const SWIPE_COMMIT_PROGRESS = 1`，调用方只引用它，两边绑死漂不开。

## 四、几条零碎但会反复踩的

- **绝对定位的子元素相对父的 border box**（不是 padding box，与 CSS 不同）。父用
  `paddingHorizontal` 时，`left: 0` 的背面卡会比正常流里的正面卡各宽出一个 inset，从两侧
  露出来。→ 父改用 `marginHorizontal`。
- **压在别的元素上的装饰层要 `pointerEvents="none"`**。RN 命中测试落到最上层视图后只往
  **祖先**上找，不会回头找兄弟节点 —— 一个 `zIndex: 1` 的纯装饰 View 会让它盖住的那条
  变成点不动的死区。`accessibilityElementsHidden` 不影响触摸路由。
- **渲染期改 ref 是副作用**。「手势只建一次但要调最新回调」的 ref 中转，赋值要放在
  `useEffect`（无依赖数组）里，不能在渲染期直接写 —— 被丢弃的那次渲染会把没生效的回调
  留在 ref 里。effect 早于任何用户触摸，读到的不会是旧的。
- **一行放不下的标签，用「换行 + 固定行高 + `overflow: hidden`」而不是不换行裁切**。
  放不下的整颗掉到第二行，而第二行完全在裁剪框外 → 要么完整、要么不出现，容器总高不变，
  且**不需要测量**。不换行只会切出半颗。
- **设计稿在溢出问题上通常帮不上忙**：示例数据都是短词，正好排满一行，从没碰到过溢出。
  别指望翻稿子能找到答案，这是实现要自己定的取舍。
