---
title: react-native-svg iOS mask 渲染空指针：转场后重绘偶现 SIGSEGV（0x0），用 pnpm patch 加防护
date: 2026-09-21
tags: troubleshooting, react-native, react-native-svg, ios, crash, pnpm-patch, luka, ips
---

# react-native-svg iOS mask 渲染空指针：转场后重绘偶现 SIGSEGV（0x0）

**症状**（2026-09-21，Luka 内测包，iPhone X / iOS 16.7）：划卡页触发订阅 → 从会员页返回 → 手指在卡面磨砂层上滑动（刮开）时**偶现闪退**。JS 日志（session.log）在崩溃前无任何 error，日志直接中断，几秒后 `luka_app_launch` —— 说明是 native 闪退。

## 怎么从 .ips 定位（可复用）

`.ips` 两段式：第一行 meta JSON（app/版本/异常摘要），第二段是 payload JSON。取证脚本：

```python
import json
d = json.loads(open(path, encoding='utf-8', errors='replace').read().split('\n', 1)[1])
print(d['exception'])          # EXC_BAD_ACCESS / SIGSEGV / subtype: KERN_INVALID_ADDRESS at 0x0
print(d['vmRegionInfo'])       # "0 is not in any region" → 典型的空指针解引用
for th in d['threads']:
    if th.get('triggered'):
        print(th.get('queue'), th.get('name'))
        for fr in th['frames'][:25]:
            print(d['usedImages'][fr['imageIndex']]['name'], hex(fr['imageOffset']), fr.get('symbol', ''))
```

本次关键栈（`Luka-2026-09-29-112959.ips`）：

```
EXC_BAD_ACCESS (SIGSEGV), KERN_INVALID_ADDRESS at 0x0, 主线程
QuartzCore   CABackingStoreUpdate_ / CA::Layer::display_
UIKitCore    -[UIView(CALayerDelegate) drawLayer:inContext:]
Luka         ×7（未 strip 无符号帧）
```

**判读**：`drawLayer:inContext:` 之下是 app 自身重写了 `drawRect:` 的视图在 CA 事务提交时执行绘制；7 个无符号帧与 `RNSVGSvgView drawRect:` → `drawToContext:` → 节点 `renderTo:` 递归渲染的调用深度吻合。当时划卡页唯一走自绘路径的组件就是 `FrostedScratch`（MaskedView + `<Svg><Mask>` + 每帧增删的 `Circle`，见 `apps/luka/src/components/discovery/frosted-scratch.tsx`）。

## 根因（react-native-svg 15.15.4）

`apple/RNSVGRenderable.mm` 的 `renderTo:rect:` 里 mask/filter 合成：

1. `CGFloat scale = [RNSVGRenderUtils getScreenScale]` → `UITraitCollection.currentTraitCollection.displayScale`，**在无 trait 环境（离屏 / 转场中重绘）返回 0**；
2. 离屏位图尺寸 `scaledWidth = rect.width × scale`、`scaledHeight = rect.height × scale`，`rect` 也可能是 0 —— 两者任一为 0 时 `CGBitmapContextCreate(...)` **返回 NULL**；
3. 之后 `CGContextConcatCTM` / `CGContextClipToRect` / … 直接对 NULL 解引用 → `SIGSEGV at 0x0`。

这解释了两个「怪」：**偶现**（只有恰好某次转场后、离屏帧被 CA 重绘才命中）和**与「会员页返回」相关**（`react-native-screens` 在 dismiss 转场时 detach/reattach 该屏，视图在这段窗口里尺寸/scale 可以退化）。

同一份报告目录里另有两份 `SIGABRT`（`com.meta.react.turbomodulemanager.queue`，`RCTExceptionsManager reportException → RCTFatal`）——那是 **JS 层错误被 fatal 化**的另一条路径，与本次 SVG 崩溃无关，需要单独复现排查。

## 修法：pnpm patch（`patches/react-native-svg@15.15.4.patch`）

三处防护，畸形帧跳过绘制（下一帧正常重绘），全部只加 guard、不改正常路径：

| 文件 | 防护 |
| --- | --- |
| `Utils/RNSVGRenderUtils.mm getScreenScale` | scale 为 0 时回落 `UIScreen.main.scale`，再为 0 则回落 `1` |
| `Utils/RNSVGRenderUtils.mm renderToImage` | `UIGraphicsGetCurrentContext()` 为 NULL 时直接返回 NULL |
| `RNSVGRenderable.mm renderTo` | filter / mask 两个分支都要求 `contentImage != NULL`；mask 分支另要求位图宽高 > 0 |

配套：`pnpm-workspace.yaml` 的 `patchedDependencies` 注册。**这是原生代码改动，必须重新出基包才生效，OTA 覆盖不到**。升级 react-native-svg 时先看上游是否已加尺寸/指针防护（检查 `RNSVGRenderable.mm` 里 mask 分支有没有 `scaledWidth > 0` 之类的判断），有则删 patch。

## 验证与遗留

- 已过：`pnpm luka:typecheck`、`pnpm lint`、`pnpm boundaries`、全量 `pnpm typecheck`、新增回归测试（paywall 去重）。
- 真机验证路径：划到当日额度用尽 → 进会员页（like / nope 各一次）→ 多次 back 回划卡页 → 立刻刮磨砂层 + 上滑/右滑，观察是否还闪退。
- 无法在本机复现断言（依赖转场时序），以重新出包后的 soak 为准。
