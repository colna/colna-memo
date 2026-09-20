---
title: RN 交叉淡入淡出「闪一下」+ 模拟器录屏验证（含 dev/HMR 坑）
date: 2026-09-20
tags: [troubleshooting, react-native, reanimated, expo, simulator, video, luka, sitin-rn]
---

# 两层视频交叉淡入淡出：小崽会「闪一下」地整帧消失

来源：2026-09-20 Luka Chats 小窝，用户报「信件视频播完的时候会闪一下再播放 stay」。

## 根因：`withTiming(条件 ? 1 : 0)` 会在过渡中途把两层同时判成「该淡出」

```tsx
// ✗ 错的：progress 在 0→1 的路上既不是 0 也不是 1
const idleStyle    = useAnimatedStyle(() => ({ opacity: withTiming(progress.value === 0 ? 1 : 0, { duration: 160 }) }));
const messageStyle = useAnimatedStyle(() => ({ opacity: withTiming(progress.value === 1 ? 1 : 0, { duration: 160 }) }));
```

`progress.value = withTiming(1, {duration: 160})` 一开始，`progress` 就是 0.37、0.62…：
两层**同时**命中「不是目标值」那一支 → 一起淡出 → 主体整帧消失（录屏实测 std≈0.5、
整块区域只剩底色），等 `progress` 精确落到 1 才淡回来。用户看到的就是「闪一下」。

**修法**：透明度直接跟着驱动值插值（和恒为 1，才是真的交叉淡入淡出），补间交给
`withTiming` 只在**驱动值那一侧**做：

```tsx
// ✓ 对的
const idleStyle    = useAnimatedStyle(() => ({ opacity: 1 - progress.value }));
const messageStyle = useAnimatedStyle(() => ({ opacity: progress.value }));
```

顺带的好处：不再每帧重建一个 `withTiming`，注入的 `reducedMotion ? 0 : 160` 也不必带进
`useAnimatedStyle`。

## 验证手法：`simctl recordVideo` + 逐帧对比度，别靠眼睛

模拟器没有 GUI 也能录（headless）：

```bash
xcrun simctl io <UDID> recordVideo --codec h264 --force /tmp/x.mov &   # 跑起来后
sleep 30 && kill -INT %1
ffmpeg -v error -i /tmp/x.mov -vf "fps=30,scale=94:203" -f image2 sf/f%04d.png
```

判据用「主体区域那块的像素标准差」：正常帧 std≈26，**主体消失的帧 std≈0.5**（区域里只剩
纯底色）。修前有 1–3 帧 std<3，修后 0 帧。顺带能确认过渡质量：过渡时刻的帧间差只有 5.2
（片段自身动作约 3），说明是平滑交叉淡入，不是硬切。

判断「过渡发生在什么时候」：让驱动信号（新消息 token）按固定间隔触发，间隔就能反推过渡
时刻；别去数帧猜。

## 两个 dev 环境的坑（这次都踩了，白测了两轮）

1. **HMR 不保证生效。** 改完文件、录屏里现象没变 ≠ 修复无效。先用**可见的探针**确认
   （把某个定位常量挪动 15%，截图看有没有动），或者干脆 terminate + relaunch 强制拉新 bundle。
2. **Metro 掉了 App 会停在 dev launcher**，这时录到的是一张静态 launcher 屏（std 恒定）。
   重新拉起 Metro 后，可以深链直接让 dev client 连上，不用点屏幕：

   ```bash
   xcrun simctl openurl <UDID> "exp+luka://expo-development-client/?url=http%3A%2F%2F192.168.x.x%3A8081"
   # 然后照常深链到具体路由： luka://dev/xxx-spike
   ```
