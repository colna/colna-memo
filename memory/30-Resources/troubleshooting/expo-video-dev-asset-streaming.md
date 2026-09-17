---
title: dev 下 expo-video 播 asset 是「边下边播」—— 卡住就先落盘
date: 2026-09-17
tags: [troubleshooting, expo-video, react-native, metro, luka]
---

# dev 下 asset 视频会边下边播，网络一抖就冻

**症状**（2026-09-17，Luka 真机 dev build）：pet 页的仓鼠视频播放时「卡住」，过几秒才继续；
1~1.5 MB 的动作片段最明显。

**根因**：`expo-video` 的 `parseSource`（`node_modules/expo-video/build/VideoPlayer.js`）对
数字 asset 走 `resolveAssetSource(id).uri` —— **dev 下这个 uri 是 Metro 的
`http://<dev-server>/assets/?unstable_path=…`**，于是 AVPlayer 拿到的是一个 HTTP 地址，
播放 = 边下边播：带宽抖一下（尤其设备与 Mac 不在同一网段时）画面就冻住。

**这也不是 expo-video 的锅**：任何「把 asset 交给原生播放器」的库在 dev 都这样；release 里
asset 是 bundle 内的本地文件，问题不存在 —— 所以它只在开发/真机调试时能看到。

**修法**（仓库里 `WelcomeVideoBackdrop` 早就是这么干的）：把 asset 先下载进 cache 再播。

```ts
const uri = Image.resolveAssetSource(moduleId)?.uri;
if (!uri?.startsWith("http")) return moduleId;          // release：本来就是本地文件
const hash = new URL(uri).searchParams.get("hash") ?? "current";
const localUri = `${FileSystem.cacheDirectory}pet-video-${hash}.mov`;
if (!(await FileSystem.getInfoAsync(localUri)).exists) await FileSystem.downloadAsync(uri, localUri);
// 播放器吃 localUri
```

要点：

- 文件名带 **asset hash**：换素材自动换缓存，不会拿到旧文件。
- 同一个 asset 只下一次（用 `Map<number, Promise>` 去重，待机与动作共享）。
- 准备期间画**静态图**顶着（Luka 是 → `pet-stage-poster` 的站立 PNG），别留空白。
- Luka 的实现：`apps/luka/src/components/pet/pet-video-source.ts`。

**顺带**：同文件里 `VideoPlayer.replace()` 在 iOS 是**主线程同步加载**（源码里带 warning，
会被弃用），无论 dev / release 都该用 `replaceAsync`。

## 相关

- [expo-video 的原生图层压不住：裁自己的容器](expo-video-native-layer-overlap.md)
- [Metro bundle 热 + 文件写入的坑](metro-bundle-hot-and-file-write.md)
